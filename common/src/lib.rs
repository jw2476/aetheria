pub mod item;
pub mod net;

use std::ops::Deref;

pub trait Observer<T> {
    fn notify(&self, old: &T, new: &T);
}

pub struct Observable<T: Clone> {
    inner: T,
    observers: Vec<Box<dyn Observer<T>>>
}

impl<T: Clone> Observable<T> {
    pub fn new(inner: T) -> Self {
        Self { inner, observers: Vec::new() }
    }

    pub fn register(&mut self, observer: Box<dyn Observer<T>>) {
        self.observers.push(observer)
    }

    pub fn run<F: FnOnce(&mut T)>(&mut self, predicate: F) {
        let old = self.inner.clone();
        predicate(&mut self.inner);
        self.observers.iter().for_each(|observer| observer.notify(&old, &self.inner))
    }

    // Chose to do this instead of DerefMut to be more verbose about the fact observers won't be
    // triggered
    pub fn run_silent<F: FnOnce(&mut T)>(&mut self, predicate: F) {
        predicate(&mut self.inner);
    }
}

impl<T: Clone> Deref for Observable<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}
