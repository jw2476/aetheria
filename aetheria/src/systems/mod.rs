use glam::Vec3;

pub mod gather;
pub mod render;

pub struct Systems<'a> {
    pub gather: &'a mut gather::System,
    pub render: &'a mut render::System,
}

pub trait Named {
    fn get_name(&self) -> String;
}

pub trait Positioned {
    fn get_position(&self) -> Vec3;
}
