use glam::UVec2;

use crate::ui::{Rectangle, Element, SizeConstraints, Region};

use super::{inventory, craft};

pub struct UI<'a> {
   pub inventory: bool,
   pub craft: bool
}

impl UI<'_> {
    pub fn new() -> Self {
        Self {
            inventory: None,
            craft: None
        }
    }
}

impl Element for UI {
    fn layout(&mut self, constraint: SizeConstraints) -> UVec2 {
        UVec2::new(constraint.max.x, constraint.max.y)
    }

    fn paint(&mut self, region: Region, scene: &mut Vec<Rectangle>) {
        if self.inventory
    }
} ff ff
