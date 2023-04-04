use std::any::{Any, TypeId};

pub use hecs_macros::*;

pub trait Scene {
    fn tick(&mut self);
    fn load() -> Self;
}

pub trait Entity: Any {}

pub trait System<T: Entity> {
    fn filter(entity: &dyn Entity) -> bool {
        println!(
            "Looking for {:?}, found {:?}",
            TypeId::of::<T>(),
            entity.type_id()
        );

        entity.type_id() == TypeId::of::<T>()
    }

    fn run(&mut self, entity: &mut T);
}
