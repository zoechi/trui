use crate::{
    geometry::{Point, Size},
    view::{Cx, View},
    widget::{
        BoxConstraints, CxState, Event, EventCx, LayoutCx, LifeCycle, LifeCycleCx, Message,
        PaintCx, Pod, PodFlags, ViewContext, WidgetState,
    },
};
use anyhow::Result;
use crossterm::{
    cursor,
    event::{
        poll, read, DisableFocusChange, DisableMouseCapture, EnableFocusChange, EnableMouseCapture,
        Event as CxEvent, KeyCode, KeyEvent,
    },
    execute,
    terminal::{
        disable_raw_mode, enable_raw_mode, BeginSynchronizedUpdate, EndSynchronizedUpdate,
        EnterAlternateScreen, LeaveAlternateScreen,
    },
};
use ratatui::{backend::CrosstermBackend, Terminal};
use std::{
    collections::HashSet,
    io::{stdout, Stdout, Write},
    path::PathBuf,
    sync::Arc,
    time::Duration,
};
use tokio::runtime::Runtime;
use tracing_subscriber::{fmt::writer::MakeWriterExt, layer::SubscriberExt, Registry};
use xilem_core::{AsyncWake, Id, IdPath, MessageResult};

// TODO less hardcoding and cross-platform support
fn setup_logging(log_level: tracing::Level) -> Result<tracing_appender::non_blocking::WorkerGuard> {
    let cache_dir = PathBuf::from(std::env::var_os("HOME").unwrap()).join(".cache/trui");
    let tracing_file_appender = tracing_appender::rolling::never(cache_dir, "trui.log");
    let (tracing_file_writer, guard) = tracing_appender::non_blocking(tracing_file_appender);

    let subscriber = Registry::default().with(
        tracing_subscriber::fmt::Layer::default()
            .with_writer(tracing_file_writer.with_max_level(log_level)),
    );
    tracing::subscriber::set_global_default(subscriber)?;

    Ok(guard)
}

pub struct App<T, V: View<T>> {
    req_chan: tokio::sync::mpsc::Sender<AppMessage>,
    render_response_chan: tokio::sync::mpsc::Receiver<RenderResponse<V, V::State>>,
    return_chan: tokio::sync::mpsc::Sender<(V, V::State, HashSet<Id>)>,
    event_chan: tokio::sync::mpsc::Receiver<Event>,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    size: Size,
    cursor_pos: Option<Point>,
    events: Vec<Message>,
    root_state: WidgetState,
    root_pod: Option<Pod>,
    cx: Cx,
    id: Option<Id>,
}

/// The standard delay for waiting for async futures.
const RENDER_DELAY: Duration = Duration::from_millis(5);

/// This is the view logic of Xilem.
///
/// It contains no information about how to interact with the User (browser, native, terminal).
/// It is created by [`App`] and kept in a separate task for updating the apps contents.
/// The App can send [AppMessage] to inform the the AppTask about an user interaction.
struct AppTask<T, V: View<T>, F: FnMut(&mut T) -> V> {
    req_chan: tokio::sync::mpsc::Receiver<AppMessage>,
    response_chan: tokio::sync::mpsc::Sender<RenderResponse<V, V::State>>,
    return_chan: tokio::sync::mpsc::Receiver<(V, V::State, HashSet<Id>)>,
    event_chan: tokio::sync::mpsc::Sender<Event>,

    data: T,
    app_logic: F,
    view: Option<V>,
    state: Option<V::State>,
    pending_async: HashSet<Id>,
    ui_state: UiState,
}

// TODO maybe rename this, so that it is clear that these events are sent to the AppTask (AppTask name is also for debate IMO)
/// A message sent from the main UI thread ([`App`]) to the [`AppTask`].
pub(crate) enum AppMessage {
    Events(Vec<Message>),
    Wake(IdPath),
    // Parameter indicates whether it should be delayed for async
    Render(bool),
}

/// A message sent from [`AppTask`] to [`App`] in response to a render request.
struct RenderResponse<V, S> {
    prev: Option<V>,
    view: V,
    state: Option<S>,
}

/// The state of the  [`AppTask`].
///
/// While the [`App`] follows a strict order of UIEvents -> Render -> Paint (this is simplified)
/// the [`AppTask`] can receive different requests at any time. This enum keeps track of the state
/// the AppTask is in because of previous requests.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum UiState {
    /// Starting state, ready for events and render requests.
    Start,
    /// Received render request, haven't responded yet.
    Delayed,
    /// An async completion woke the UI thread.
    WokeUI,
}

impl<T: Send + 'static, V: View<T> + 'static> App<T, V> {
    pub fn new(data: T, app_logic: impl FnMut(&mut T) -> V + Send + 'static) -> Self {
        let backend = CrosstermBackend::new(stdout());
        let terminal = Terminal::new(backend).unwrap(); // TODO handle errors...

        // Create a new tokio runtime. Doing it here is hacky, we should allow
        // the client to do it.
        let rt = Runtime::new().unwrap();

        // Note: there is danger of deadlock if exceeded; think this through.
        const CHANNEL_SIZE: usize = 1000;
        let (message_tx, message_rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (event_tx, event_rx) = tokio::sync::mpsc::channel(CHANNEL_SIZE);
        let (response_tx, response_rx) = tokio::sync::mpsc::channel(1);
        let (return_tx, return_rx) = tokio::sync::mpsc::channel(1);

        // We have a separate thread to forward wake requests (mostly generated
        // by the custom waker when we poll) to the async task. Maybe there's a
        // better way, but this is expedient.
        //
        // It's a sync_channel because sender needs to be sync to work in an async
        // context. Consider crossbeam and flume channels as alternatives.
        let message_tx_clone = message_tx.clone();
        let (wake_tx, wake_rx) = std::sync::mpsc::sync_channel(10);
        std::thread::spawn(move || {
            while let Ok(id_path) = wake_rx.recv() {
                let _ = message_tx_clone.blocking_send(AppMessage::Wake(id_path));
            }
        });

        // spawn io event proxy task
        let event_tx_clone = event_tx.clone();
        std::thread::spawn(move || {
            loop {
                if let Ok(true) = poll(Duration::from_millis(100)) {
                    let event = match read() {
                        // TODO quit app at least for now, until proper key handling is implemented, then this thread might need a signal to quit itself
                        Ok(CxEvent::Key(KeyEvent {
                            code: KeyCode::Esc, ..
                        })) => Event::Quit,
                        Ok(CxEvent::Key(key_event)) => Event::Key(key_event),
                        Ok(CxEvent::Mouse(mouse_event)) => Event::Mouse(mouse_event),
                        Ok(CxEvent::FocusGained) => Event::FocusGained,
                        Ok(CxEvent::FocusLost) => Event::FocusLost,
                        // CxEvent::Paste(_) => todo!(),
                        Ok(CxEvent::Resize(width, height)) => Event::Resize { width, height },
                        _ => continue, // TODO handle other kinds of events and errors
                    };

                    let quit = matches!(event, Event::Quit);

                    let _ = event_tx_clone.blocking_send(event);

                    if quit {
                        break;
                    }
                }
            }
        });

        // Send this event here, so that the app renders directly when it is run.
        let _ = event_tx.blocking_send(Event::Start);

        // spawn app task
        rt.spawn(async move {
            let mut app_task = AppTask {
                req_chan: message_rx,
                response_chan: response_tx,
                return_chan: return_rx,
                event_chan: event_tx,
                data,
                app_logic,
                view: None,
                state: None,
                pending_async: HashSet::new(),
                ui_state: UiState::Start,
            };
            app_task.run().await;
        });

        let cx = Cx::new(&wake_tx, Arc::new(rt));

        App {
            req_chan: message_tx,
            render_response_chan: response_rx,
            return_chan: return_tx,
            event_chan: event_rx,
            terminal,
            size: Size::default(),
            cursor_pos: None,
            root_pod: None,
            cx,
            id: None,
            root_state: WidgetState::new(),
            events: Vec::new(),
        }
    }

    fn send_events(&mut self) {
        if !self.events.is_empty() {
            let events = std::mem::take(&mut self.events);
            let _ = self.req_chan.blocking_send(AppMessage::Events(events));
        }
    }

    /// Run the app logic and update the widget tree.
    #[tracing::instrument(skip(self))]
    fn render(&mut self) -> Result<()> {
        if self.build_widget_tree(false) {
            self.build_widget_tree(true);
        }
        let root_pod = self.root_pod.as_mut().unwrap();

        let cx_state = &mut CxState::new(&mut self.events);

        // TODO via event (Event::Resize)?
        self.terminal.autoresize()?;

        let term_rect = self.terminal.size()?;
        let ratatui::layout::Rect { width, height, .. } = term_rect;
        let term_size = Size {
            width: width as f64,
            height: height as f64,
        };
        tracing::error!("current size: {term_size}");

        let needs_layout_recomputation = root_pod
            .state
            .flags
            .intersects(PodFlags::REQUEST_LAYOUT | PodFlags::TREE_CHANGED)
            || term_size != self.size;

        if needs_layout_recomputation {
            let _ = tracing::debug_span!("compute layout");
            self.size = term_size;
            let mut layout_cx = LayoutCx {
                widget_state: &mut self.root_state,
                cx_state,
            };
            let bc = BoxConstraints::tight(self.size);
            root_pod.layout(&mut layout_cx, &bc);
            root_pod.set_origin(&mut layout_cx, Point::ORIGIN);
        }
        if root_pod
            .state
            .flags
            .contains(PodFlags::VIEW_CONTEXT_CHANGED)
        {
            let view_context = ViewContext {
                window_origin: Point::ORIGIN,
                // clip: Rect::from_origin_size(Point::ORIGIN, root_pod.state.size),
                mouse_position: self.cursor_pos,
            };
            let mut lifecycle_cx = LifeCycleCx {
                cx_state,
                widget_state: &mut self.root_state,
            };
            root_pod.lifecycle(
                &mut lifecycle_cx,
                &LifeCycle::ViewContextChanged(view_context),
            );
        }

        if root_pod.state.flags.intersects(PodFlags::REQUEST_PAINT) || needs_layout_recomputation {
            let _paint_span = tracing::debug_span!("paint");
            let mut paint_cx = PaintCx {
                widget_state: &mut self.root_state,
                cx_state,
                terminal: &mut self.terminal,
                override_style: ratatui::style::Style::default(),
            };

            root_pod.paint(&mut paint_cx);
            execute!(stdout(), BeginSynchronizedUpdate)?;
            self.terminal.flush()?;
            execute!(stdout(), EndSynchronizedUpdate)?;
            self.terminal.swap_buffers();
            self.terminal.backend_mut().flush()?;
        }
        Ok(())
    }

    /// Run one pass of app logic.
    ///
    /// Return value is whether there are any pending async futures.
    fn build_widget_tree(&mut self, delay: bool) -> bool {
        self.cx.pending_async.clear();
        let _ = self.req_chan.blocking_send(AppMessage::Render(delay));
        if let Some(response) = self.render_response_chan.blocking_recv() {
            let state = if let Some(widget) = self.root_pod.as_mut() {
                let mut state = response.state.unwrap();
                let changes = response.view.rebuild(
                    &mut self.cx,
                    response.prev.as_ref().unwrap(),
                    self.id.as_mut().unwrap(),
                    &mut state,
                    //TODO: fail more gracefully but make it explicit that this is a bug
                    widget
                        .downcast_mut()
                        .expect("the root widget changed its type, this should never happen!"),
                );
                let _ = self.root_pod.as_mut().unwrap().mark(changes);
                assert!(self.cx.is_empty(), "id path imbalance on rebuild");
                state
            } else {
                let (id, state, widget) = response.view.build(&mut self.cx);
                assert!(self.cx.is_empty(), "id path imbalance on build");
                self.root_pod = Some(Pod::new(widget));
                self.id = Some(id);
                state
            };
            let pending = std::mem::take(&mut self.cx.pending_async);
            let has_pending = !pending.is_empty();
            let _ = self
                .return_chan
                .blocking_send((response.view, state, pending));
            has_pending
        } else {
            false
        }
    }

    pub fn run(mut self) -> Result<()> {
        let _guard = setup_logging(tracing::Level::DEBUG)?;

        enable_raw_mode()?;
        execute!(
            stdout(),
            EnterAlternateScreen,
            EnableFocusChange,
            EnableMouseCapture,
            cursor::Hide
        )?;

        self.terminal.clear()?;

        let main_loop_tracing_span = tracing::debug_span!("main loop");
        while let Some(event) = self.event_chan.blocking_recv() {
            let mut events = vec![event];
            // batch events
            while let Ok(event) = self.event_chan.try_recv() {
                events.push(event);
            }

            let quit = events.iter().any(|e| matches!(e, Event::Quit));

            if let Some(root_pod) = self.root_pod.as_mut() {
                let cx_state = &mut CxState::new(&mut self.events);

                let mut cx = EventCx {
                    is_handled: false,
                    widget_state: &mut self.root_state,
                    cx_state,
                };
                for event in events {
                    // TODO filter out some events like Event::Wake?
                    root_pod.event(&mut cx, &event);
                }
            }
            self.send_events();

            self.render()?;
            if quit {
                break;
            }
        }
        drop(main_loop_tracing_span);

        execute!(
            stdout(),
            cursor::Show,
            LeaveAlternateScreen,
            DisableFocusChange,
            DisableMouseCapture
        )?;
        disable_raw_mode()?;
        Ok(())
    }
}

impl<T, V: View<T>, F: FnMut(&mut T) -> V> AppTask<T, V, F> {
    async fn run(&mut self) {
        let mut deadline = None;
        loop {
            let rx = self.req_chan.recv();
            let req = match deadline {
                Some(deadline) => tokio::time::timeout_at(deadline, rx).await,
                None => Ok(rx.await),
            };
            match req {
                Ok(Some(req)) => match req {
                    AppMessage::Events(events) => {
                        for event in events {
                            let id_path = &event.id_path[1..];
                            self.view.as_ref().unwrap().message(
                                id_path,
                                self.state.as_mut().unwrap(),
                                event.body,
                                &mut self.data,
                            );
                        }
                    }
                    AppMessage::Wake(id_path) => {
                        let needs_rebuild;
                        {
                            let result = self.view.as_ref().unwrap().message(
                                &id_path[1..],
                                self.state.as_mut().unwrap(),
                                Box::new(AsyncWake),
                                &mut self.data,
                            );
                            needs_rebuild = matches!(result, MessageResult::RequestRebuild);
                            tracing::debug!("Needs rebuild after wake: {needs_rebuild}");
                        }

                        if needs_rebuild {
                            // request re-render from UI thread
                            if self.ui_state == UiState::Start {
                                self.ui_state = UiState::WokeUI;
                                tracing::debug!("Sending wake event");
                                if self.event_chan.send(Event::Wake).await.is_err() {
                                    break;
                                }
                            }
                            let id = id_path.last().unwrap();
                            self.pending_async.remove(id);
                            if self.pending_async.is_empty() && self.ui_state == UiState::Delayed {
                                tracing::debug!("Render with delayed ui state");
                                self.render().await;
                                deadline = None;
                            }
                        }
                    }
                    AppMessage::Render(delay) => {
                        if !delay || self.pending_async.is_empty() {
                            tracing::debug!("Render without delay");
                            self.render().await;
                            deadline = None;
                        } else {
                            tracing::debug!(
                                "Pending async, delay rendering by {} us",
                                RENDER_DELAY.as_micros()
                            );
                            deadline = Some(tokio::time::Instant::now() + RENDER_DELAY);
                            self.ui_state = UiState::Delayed;
                        }
                    }
                },
                Ok(None) => break,
                Err(_) => {
                    tracing::debug!("Render after delay");
                    self.render().await;
                    deadline = None;
                }
            }
        }
    }

    async fn render(&mut self) {
        let view = (self.app_logic)(&mut self.data);
        let response = RenderResponse {
            prev: self.view.take(),
            view,
            state: self.state.take(),
        };
        if self.response_chan.send(response).await.is_err() {
            tracing::error!("error sending render response");
        }
        if let Some((view, state, pending)) = self.return_chan.recv().await {
            self.view = Some(view);
            self.state = Some(state);
            self.pending_async = pending;
        }
        self.ui_state = UiState::Start;
    }
}
