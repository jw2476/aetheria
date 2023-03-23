#![feature(trait_upcasting)]

use std::{
    any::Any,
    ops::{Deref, DerefMut},
};

pub struct Entity {
    components: Vec<Box<dyn Component>>,
}

impl Entity {
    pub fn get<T: Component>(&self) -> Option<&T> {
        let component: &Box<dyn Component> = self
            .components
            .iter()
            .find(|&component| component.get_id() == T::id())?;

        let component: &dyn Component = component.deref();
        <dyn Any>::downcast_ref(component as &dyn Any)
    }

    pub fn get_mut<T: Component>(&mut self) -> Option<&mut T> {
        let component: &mut Box<dyn Component> = self
            .components
            .iter_mut()
            .find(|component| component.get_id() == T::id())?;

        <dyn Any>::downcast_mut(component.deref_mut() as &mut dyn Any)
    }

    pub fn add(&mut self, component: Box<dyn Component>) {
        self.components.push(component)
    }
}

pub trait Component: Any {
    fn get_id(&self) -> u128;
    fn id() -> u128
    where
        Self: Sized;
}

pub trait System {
    fn get_requirements(&self) -> u128;

    fn _check(&self, entity: &Entity) -> bool {
        let requirements = self.get_requirements();
        let hash = entity
            .components
            .iter()
            .map(|component| component.get_id())
            .fold(0, |hash, id| hash | id);
        (hash & requirements) == requirements
    }

    fn run(&mut self, entity: &mut Entity);
    fn handle(&mut self, event: Event) {}
}

#[derive(Copy, Clone, Debug)]
pub enum Event {
    WindowResized,
    CloseRequested
}

#[derive(Default)]
pub struct World {
    entities: Vec<Entity>,
    systems: Vec<Box<dyn System>>,
}

impl World {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_system(&mut self, system: Box<dyn System>) {
        self.systems.push(system)
    }

    pub fn spawn(&mut self) -> &mut Entity {
        let entity = Entity {
            components: Vec::new(),
        };
        self.entities.push(entity);

        self.entities.last_mut().unwrap()
    }

    pub fn tick(&mut self) {
        for system in self.systems.iter_mut() {
            for entity in self.entities.iter_mut() {
                if system._check(entity) {
                    system.run(entity);
                }
            }
        }
    }

    pub fn dispatch_event(&mut self, event: Event) {
        for system in self.systems.iter_mut() {
            system.handle(event);
        }
    }
}

