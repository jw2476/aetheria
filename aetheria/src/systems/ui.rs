use std::sync::{Weak, Mutex, Arc};

pub struct System {
    generators: Vec<Weak<Mutex<dyn UIGenerator>>>
}

impl System {
    pub fn new() -> Self {
        Self { generators: Vec::new() }
    }

    pub fn add<T: UIGenerator + Sized + 'static>(&mut self, generator: Arc<Mutex<T>>) {
        self.generators.push(Arc::downgrade(
            &(generator as Arc<Mutex<dyn UIGenerator>>),
        ))
    }
}

pub trait UIGenerator {
    fn generate() ff
} ff
