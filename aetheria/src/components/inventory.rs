use glam::{UVec2, Vec4};

use crate::ui::{Element, Rectangle, Region, SizeConstraints};
use common::item::Inventory;

use super::components::{Container, Padding, Text};

#[derive(Clone, Debug)]
pub enum HAlign {
    Left,
    Right,
    Center,
}

#[derive(Clone, Debug)]
pub struct VList<T: Element> {
    pub children: Vec<T>,
    pub separation: u32,
    pub align: HAlign,
}

// TODO: Alignment
impl<T: Element> Element for VList<T> {
    fn layout(&mut self, constraint: SizeConstraints) -> UVec2 {
        if self.children.len() == 0 {
            return UVec2::new(0, 0);
        }

        let children_sizes = self
            .children
            .iter_mut()
            .map(|child| child.layout(constraint.clone()))
            .collect::<Vec<UVec2>>();
        let width = children_sizes.iter().map(|size| size.x).max().unwrap();
        let height = children_sizes.first().unwrap().y * self.children.len() as u32
            + self.separation * (self.children.len() as u32 - 1);

        UVec2 {
            x: width,
            y: height,
        }
    }

    fn paint(&mut self, region: Region, scene: &mut Vec<Rectangle>) {
        if self.children.len() == 0 {
            return;
        }

        let height_per_child = (region.size.y + self.separation
            - (self.children.len() as u32 * self.separation))
            / (self.children.len() as u32);

        for (i, child) in self.children.iter_mut().enumerate() {
            child.paint(
                Region {
                    origin: region.origin
                        + UVec2::new(0, (height_per_child + self.separation) * i as u32),
                    size: UVec2::new(region.size.x, height_per_child),
                },
                scene,
            );
        }
    }
}

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
