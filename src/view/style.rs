use std::any::{type_name, Any, TypeId};
use std::collections::HashMap;
use std::fmt::{Debug, Display};

use std::sync::Arc;

use crate::widget::{Block, Widget};

#[derive(Debug)]
pub struct Styles {
    styles: Vec<Arc<dyn Style>>,
    styles_by_id: HashMap<Arc<str>, Vec<Arc<dyn Style>>>,
    styles_by_widget_type_id: HashMap<TypeId, Vec<Arc<dyn Style>>>,
    styles_by_class: HashMap<Arc<str>, Vec<Arc<dyn Style>>>,
    styles_by_attribute: HashMap<Arc<str>, Vec<Arc<dyn Style>>>,
}

impl Styles {
    pub fn new(styles: Vec<Arc<dyn Style>>) -> Self {
        let mut styles = Self {
            styles,
            styles_by_id: HashMap::new(),
            styles_by_widget_type_id: HashMap::new(),
            styles_by_class: HashMap::new(),
            styles_by_attribute: HashMap::new(),
        };
        styles.styles.iter().inspect(|s| {
            s.id().into_iter().inspect(|id| {
                if let Some(v) = styles.styles_by_id.get_mut(id) {
                    v.push((*s).clone());
                } else {
                    styles
                        .styles_by_id
                        .insert((*id).clone(), vec![(*s).clone()]);
                }
            });
        });

        styles
    }
}

impl Display for Styles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?})", self.styles)
    }
}

#[derive(Debug)]
pub struct StyleBase<T: Widget> {
    position: u16,
    selectivity: u8,
    // widget_type_id: TypeId,
    id: Option<Arc<str>>,
    classes: Vec<Arc<str>>,
    attributes: Vec<Box<dyn Attribute<Widget = T>>>,
}

impl<T: Widget> Display for StyleBase<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {}, {:?}, {:?}, {:?})",
            self.position,
            self.selectivity,
            type_name::<T>(),
            self.id,
            self.classes,
            self.attributes
        )
    }
}

pub struct AttributeBase<T: Widget> {
    name: Arc<str>,
    comparison: Box<dyn Fn(T) -> bool>,
}

impl<T: Widget> Debug for AttributeBase<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(&format!("AttributeBase<{}>", type_name::<T>()))
            .field("name", &self.name)
            .field("comparison", &"<closure>")
            .finish()
    }
}

pub trait Attribute: Display + Debug {
    type Widget;
}

pub trait AnyAttribute: Attribute<Widget = dyn Widget> + Display + Debug {
    fn as_any(&self) -> &dyn Any;

    fn as_any_mut(&mut self) -> &mut dyn Any;

    fn type_name(&self) -> &'static str;

    // fn name(&self) -> Arc<str>;
}

impl<A: Attribute<Widget = dyn Widget>> AnyAttribute for A {
    fn as_any(&self) -> &dyn Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn Any {
        self
    }

    fn type_name(&self) -> &'static str {
        std::any::type_name::<Self>()
    }
}

// pub trait Attribute<T: Widget> : Display + Debug {
//     fn name(&self) -> Arc<str>;
//     fn compare(&self, widget: T) -> bool;
// }

pub trait Style: Debug + Display {
    fn position(&self) -> u16;
    fn selectivity(&self) -> u8;
    fn widget_type_id(&self) -> TypeId;
    fn id(&self) -> Option<Arc<str>>;
    fn classes(&self) -> &Vec<Arc<str>>;
    fn attributes(&self) -> &Vec<Box<dyn AnyAttribute>>;
}

pub struct BlockStyle {
    base: StyleBase<Block>,
}

impl BlockStyle {
    pub fn new(
        id: Option<Arc<str>>,
        classes: Vec<Arc<str>>,
        attributes: Vec<Box<dyn Attribute<Widget = Block>>>,
    ) -> Self {
        Self {
            base: StyleBase::<Block> {
                position: 0,
                selectivity: 0,
                // widget_type_id: TypeId::of::<Block>(),
                id,
                classes,
                attributes,
            },
        }
    }
}

impl Debug for BlockStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct(type_name::<BlockStyle>())
            .field("base", &self.base)
            .finish()
    }
}

impl Display for BlockStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "({:?})", self.base)
    }
}

impl Style for BlockStyle {
    fn position(&self) -> u16 {
        self.base.position
    }

    fn selectivity(&self) -> u8 {
        self.base.selectivity
    }

    fn widget_type_id(&self) -> TypeId {
        TypeId::of::<BlockStyle>()
    }

    fn id(&self) -> Option<Arc<str>> {
        self.base.id.clone()
    }

    fn classes(&self) -> &Vec<Arc<str>> {
        &self.base.classes
    }

    fn attributes(&self) -> &Vec<Box<dyn AnyAttribute>> {
        &self.base.attributes
    }
}

// pub trait AnyStyle: Style {
//     fn as_any(&self) -> &dyn Any;

//     fn as_any_mut(&mut self) -> &mut dyn Any;

//     fn type_name(&self) -> &'static str;
// }

// impl<S: Style> AnyStyle for S {
//     fn as_any(&self) -> &dyn Any {
//         self
//     }

//     fn as_any_mut(&mut self) -> &mut dyn Any {
//         self
//     }

//     fn type_name(&self) -> &'static str {
//         std::any::type_name::<Self>()
//     }
// }

// impl Style for Box<dyn AnyStyle> {
//     fn position(&self) -> u16 {
//         Style::position(self)
//     }

//     fn selectivity(&self) -> u8 {
//         Style::selectivity(self)
//     }

//     fn widget_type_id(&self) -> TypeId {
//         Style::widget_type_id(self)
//     }

//     fn id(&self) -> Option<Arc<str>> {
//         Style::id(self)
//     }

//     fn classes(&self) -> &Vec<Arc<str>> {
//         Style::classes(self)
//     }

//     fn attributes(&self) -> &Vec<Box<dyn AnyAttribute>> {
//         Style::classes(self)
//     }
// }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_test() {
        let styles = Styles::new(BlockStyle::new());
    }
}
