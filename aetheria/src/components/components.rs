use crate::ui::{Element, SizeConstraints, Region, Rectangle, CHAR_WIDTH, CHAR_HEIGHT};
use glam::{Vec4, UVec2};

#[derive(Clone, Debug)]
pub struct Container<T: Element> {
    pub child: T,
    pub color: Vec4,
    pub border_radius: u32,
    pub border_color: Vec4,
}

impl<T: Element> Element for Container<T> {
    fn layout(&mut self, constraint: SizeConstraints) -> UVec2 {
        let max = constraint.max - UVec2::new(self.border_radius, self.border_radius);
        let child_size = self.child.layout(SizeConstraints {
            min: constraint.min,
            max,
        });

        child_size + UVec2::new(self.border_radius * 2, self.border_radius * 2)
    }

    fn paint(&mut self, region: Region, scene: &mut Vec<Rectangle>) {
        scene.push(Rectangle {
            color: self.border_color,
            origin: region.origin,
            extent: region.size,
            radius: self.border_radius,
            ..Default::default()
        });
        scene.push(Rectangle {
            color: self.color,
            origin: region.origin + UVec2::new(self.border_radius, self.border_radius),
            extent: region.size - UVec2::new(self.border_radius * 2, self.border_radius * 2),
            ..Default::default()
        });

        self.child.paint(
            Region {
                origin: region.origin + UVec2::new(self.border_radius, self.border_radius),
                size: region.size - UVec2::new(self.border_radius * 2, self.border_radius * 2),
            },
            scene,
        )
    }
}

#[derive(Clone, Debug)]
pub struct Padding<T: Element> {
    pub child: T,
    pub top: u32,
    pub bottom: u32,
    pub left: u32,
    pub right: u32,
}

impl<T: Element> Element for Padding<T> {
    fn layout(&mut self, constraint: SizeConstraints) -> UVec2 {
        let max = constraint.max - UVec2::new(self.left + self.right, self.top + self.bottom);
        let child_size = self.child.layout(SizeConstraints {
            min: constraint.min,
            max,
        });

        child_size + UVec2::new(self.left + self.right, self.top + self.bottom)
    }

    fn paint(&mut self, region: Region, scene: &mut Vec<Rectangle>) {
        self.child.paint(
            Region {
                origin: region.origin + UVec2::new(self.left, self.top),
                size: region.size - UVec2::new(self.left + self.right, self.top + self.bottom),
            },
            scene,
        );
    }
}

#[derive(Clone, Copy, Debug)]
pub enum VAlign {
    Top,
    Bottom,
    Center,
}

#[derive(Clone, Debug)]
pub struct HPair<L: Element, R: Element> {
    pub left: L,
    pub right: R,
    pub align: VAlign,
    pub separation: u32,
    left_size: UVec2,
    right_size: UVec2,
}

impl<L: Element, R: Element> HPair<L, R> {
    pub fn new(left: L, right: R, align: VAlign, separation: u32) -> Self {
        Self {
            left,
            right,
            align,
            separation,
            left_size: UVec2::ZERO,
            right_size: UVec2::ZERO,
        }
    }

    fn get_top_padding(&self, wanted: u32, actual: u32) -> u32 {
        println!("Wanted: {}, Actual: {}", wanted, actual);
        match self.align {
            VAlign::Top => 0,
            VAlign::Bottom => wanted - actual,
            VAlign::Center => (wanted - actual) / 2,
        }
    }

    fn get_bottom_padding(&self, wanted: u32, actual: u32) -> u32 {
        (wanted - actual) - self.get_top_padding(wanted, actual)
    }
}

impl<L: Element, R: Element> Element for HPair<L, R> {
    fn layout(&mut self, constraint: SizeConstraints) -> UVec2 {
        self.left_size = self.left.layout(constraint.clone());
        self.right_size = self.right.layout(SizeConstraints {
            min: constraint.min,
            max: constraint.max - UVec2::new(self.left_size.x + self.separation, 0),
        });

        UVec2::new(
            self.left_size.x + self.right_size.x + self.separation,
            *[self.left_size.y, self.right_size.y].iter().max().unwrap(),
        )
    }

    fn paint(&mut self, region: Region, scene: &mut Vec<Rectangle>) {
        let mut left = Padding {
            child: self.left.clone(),
            top: self.get_top_padding(region.size.y, self.left_size.y),
            bottom: self.get_bottom_padding(region.size.y, self.left_size.y),
            left: 0,
            right: 0,
        };
        left.paint(
            Region {
                origin: region.origin,
                size: UVec2::new(self.left_size.x, region.size.y),
            },
            scene,
        );

        let mut right = Padding {
            child: self.right.clone(),
            top: self.get_top_padding(region.size.y, self.right_size.y),
            bottom: self.get_bottom_padding(region.size.y, self.right_size.y),
            left: 0,
            right: 0,
        };
        right.paint(
            Region {
                origin: region.origin + UVec2::new(self.left_size.x + self.separation, 0),
                size: UVec2::new(self.right_size.x, region.size.y),
            },
            scene,
        );
    }
}

static ASCII_UPPER: [char; 37] = [
    'A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I', 'J', 'K', 'L', 'M', 'N', 'O', 'P', 'Q', 'R', 'S',
    'T', 'U', 'V', 'W', 'X', 'Y', 'Z', ' ', '0', '1', '2', '3', '4', '5', '6', '7', '8', '9',
];

#[derive(Clone, Debug)]
pub struct Text {
    pub color: Vec4,
    pub content: String,
}

impl Element for Text {
    fn layout(&mut self, constraint: SizeConstraints) -> UVec2 {
        UVec2::new(
            (self.content.len() as u32 * CHAR_WIDTH).max(constraint.min.x),
            CHAR_HEIGHT.max(constraint.min.y),
        )
    }

    fn paint(&mut self, region: Region, scene: &mut Vec<Rectangle>) {
        for (i, c) in self.content.to_uppercase().chars().enumerate() {
            scene.push(Rectangle {
                color: self.color,
                origin: region.origin + UVec2::new(CHAR_WIDTH * i as u32, 0),
                extent: UVec2::new(CHAR_HEIGHT, 5),
                atlas_id: ASCII_UPPER
                    .iter()
                    .position(|a| *a == c)
                    .expect(&format!("Character {} not in font", c))
                    as i32,
                ..Default::default()
            })
        }
    }
}
