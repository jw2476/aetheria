use glam::Vec3;

pub mod interact;
pub mod render;

pub struct Systems<'a> {
    pub interact: &'a mut interact::System,
    pub render: &'a mut render::System,
}

pub trait Named {
    fn get_name(&self) -> String;
}

pub trait Positioned {
    fn get_position(&self) -> Vec3;
}
