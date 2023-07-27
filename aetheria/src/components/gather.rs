use glam::Vec4;
use super::components::*;


pub type Component = Container<Padding<HPair<Container<Padding<Text>>, Text>>>;

impl Component {
    pub fn new() -> Self {
        let f = Text {
            color: Vec4::new(0.957, 0.247, 0.369, 1.0),
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
            color: Vec4::new(0.094, 0.094, 0.106, 1.0),
            border_color: Vec4::new(0.957, 0.247, 0.369, 1.0),
            border_radius: 1,
        };
        let right = Text {
            color: Vec4::new(0.957, 0.247, 0.369, 1.0),
            content: "Gather".to_owned(),
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
            border_color: Vec4::new(0.957, 0.247, 0.369, 1.0),
            color: Vec4::new(0.094, 0.094, 0.106, 1.0),
        }
        .into()
    }
}
