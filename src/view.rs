mod border;
mod common;
mod core;
mod defer;
mod events;
mod fill_max_size;
mod linear_layout;
mod margin;
mod style;
mod text;
mod use_state;
mod weighted_linear_layout;

use std::marker::PhantomData;

pub use xilem_core::{Id, IdPath, VecSplice};

// TODO do this via a prelude instead (and possibly not wildcard export)
pub use self::core::*;
pub use border::*;
pub use common::*;
pub use defer::*;
pub use events::*;
pub use fill_max_size::*;
pub use linear_layout::*;
pub use margin::*;
pub use style::*;
pub use text::*;
pub use use_state::*;
pub use weighted_linear_layout::*;

// TODO this could maybe also be added directly to `View` (possibly copying the macro expanded version of it)
/// A trait that makes it possible to use core views such as [`Adapt`] in the continuation/builder style.
pub trait ViewExt<T, A>: View<T, A> + Sized {
    fn adapt<ParentT, ParentA, F>(self, f: F) -> Adapt<ParentT, ParentA, T, A, Self, F>
    where
        F: Fn(&mut ParentT, AdaptThunk<T, A, Self>) -> xilem_core::MessageResult<ParentA>
            + Sync
            + Send,
    {
        Adapt::new(f, self)
    }

    fn adapt_state<ParentT, F>(self, f: F) -> AdaptState<ParentT, T, Self, F>
    where
        F: Fn(&mut ParentT) -> &mut T + Send + Sync + Send,
    {
        AdaptState::new(f, self)
    }

    fn margin<S: Into<MarginStyle>>(self, style: S) -> Margin<Self, T, A> {
        let style = style.into();
        Margin {
            content: self,
            position: style.position,
            amount: style.amount,
            phantom: PhantomData,
        }
    }

    fn border<S: Into<BorderStyle>>(self, style: S) -> Border<Self, T, A> {
        let style = style.into();
        Border {
            content: self,
            borders: style.borders,
            kind: style.kind,
            style: style.style,
            phantom: PhantomData,
        }
    }

    fn fill_max_size<S: Into<FillMaxSizeStyle>>(self, style: S) -> FillMaxSize<Self, T, A> {
        let style = style.into();
        FillMaxSize {
            content: self,
            fill: style.fill,
            percent: style.percent,
            phantom: PhantomData,
        }
    }

    fn fill_max_width(self, percent: f64) -> FillMaxSize<Self, T, A> {
        FillMaxSize {
            content: self,
            fill: Fill::WIDTH,
            percent,
            phantom: PhantomData,
        }
    }

    fn fill_max_height(self, percent: f64) -> FillMaxSize<Self, T, A> {
        FillMaxSize {
            content: self,
            fill: Fill::HEIGHT,
            percent,
            phantom: PhantomData,
        }
    }

    fn on_click<EH: EventHandler<T, A>>(self, event_handler: EH) -> OnClick<Self, EH> {
        OnClick {
            view: self,
            event_handler,
        }
    }

    fn weight(self, weight: f64) -> WeightedLayoutElement<Self, T, A> {
        WeightedLayoutElement {
            content: self,
            weight,
            phantom: PhantomData,
        }
    }

    fn on_mouse<EH: EventHandler<T, A, crate::widget::MouseEvent>>(
        self,
        event_handler: EH,
    ) -> OnMouse<Self, EH> {
        OnMouse {
            view: self,
            catch_event: crate::CatchMouseButton::empty(),
            event_handler,
        }
    }

    fn on_hover<EH: EventHandler<T, A>>(self, event_handler: EH) -> OnHover<Self, EH> {
        OnHover {
            view: self,
            event_handler,
        }
    }

    fn on_blur_hover<EH: EventHandler<T, A>>(self, event_handler: EH) -> OnHoverLost<Self, EH> {
        OnHoverLost {
            view: self,
            event_handler,
        }
    }
}

impl<T, A, V: View<T, A>> ViewExt<T, A> for V {}
