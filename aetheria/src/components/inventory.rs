use glam::{UVec2, Vec4};

use super::components::{Container, HAlign, Padding, Text, VList};
use crate::{
    data::inventory::Inventory,
    ui::{Element, Rectangle, Region, SizeConstraints},
};

pub type Component = Container<Padding<VList<Text>>>;

impl Component {
    pub fn new(inventory: &Inventory) -> Self {
        let text = inventory
            .get_items()
            .iter()
            .map(|stack| Text {
                color: Self::get_highlight(),
                content: format!("{:?} x{}", stack.item, stack.amount),
            })
            .collect::<Vec<Text>>();
        let vlist = VList {
            children: text,
            separation: 3,
            align: HAlign::Left,
        };
        let padding = Padding::new_uniform(vlist, 2);
        Self {
            child: padding,
            color: Self::get_background(),
            border_radius: 1,
            border_color: Self::get_highlight(),
        }
    }
}
