use crate::ui::Element;

use super::components::*;
use glam::Vec4;

pub type Component = Container<Padding<HPair<Container<Padding<Text>>, Text>>>;

impl Component {
    pub fn new(name: &str) -> Self {
        let f = Text {
            color: Self::get_highlight(),
            content: "F".to_owned(),
        };
        let padded_f = Padding {
            child: f,
            top: 1,
            bottom: 1,
            left: 1,
            right: 0,
        };
        let left = Container {
            child: padded_f,
            color: Self::get_background(),
            border_color: Self::get_highlight(),
            border_radius: 1,
        };
        let right = Text {
            color: Self::get_highlight(),
            content: name.to_owned(),
        };
        let hpair = HPair::new(left, right, VAlign::Center, 2);
        let padding = Padding {
            child: hpair,
            top: 2,
            bottom: 2,
            left: 2,
            right: 2,
        };
        Container {
            child: padding,
            border_radius: 1,
            border_color: Self::get_highlight(),
            color: Self::get_background(),
        }
        .into()
    }
}
