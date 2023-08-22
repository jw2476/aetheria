use common::item::ItemStack;

use super::components::{Container, HAlign, Padding, Text, VList, VPair};
use crate::ui::Element;
use glam::Vec4;

pub struct Recipe {
    pub ingredients: Vec<ItemStack>,
    pub outputs: Vec<ItemStack>,
}

pub type Component = Container<Padding<VPair<VList<Text>, Container<Padding<Text>>>>>;

impl Component {
    pub fn new(recipe: Recipe) -> Self {
        let button = Container {
            child: Padding::new_uniform(
                Text {
                    color: Self::get_highlight(),
                    content: "Craft".to_owned(),
                },
                3,
            ),
            color: Self::get_background(),
            border_color: Self::get_highlight(),
            border_radius: 1,
        };

        let mut text = Vec::new();
        recipe.ingredients.iter().for_each(|ingredient| {
            text.push(Text {
                color: Self::get_highlight(),
                content: format!("{}", ingredient),
            })
        });
        text.push(Text {
            color: Vec4::ZERO,
            content: String::new(),
        });
        recipe.outputs.iter().for_each(|output| {
            text.push(Text {
                color: Self::get_highlight(),
                content: format!("{}", output),
            })
        });

        let text = VList {
            children: text,
            separation: 2,
            align: HAlign::Left,
        };

        let pair = VPair::new(text, button, HAlign::Center, 6);

        Self {
            child: Padding::new_uniform(pair, 2),
            color: Self::get_background(),
            border_color: Self::get_highlight(),
            border_radius: 1,
        }
    }
}
