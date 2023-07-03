use std::collections::{HashMap, HashSet};
use glam::Vec2;

#[derive(Default)]
pub struct Keyboard {
    down: HashSet<winit::event::VirtualKeyCode>,
    pressed: HashSet<winit::event::VirtualKeyCode>
}

impl Keyboard {
    pub fn new() -> Self {
        Self::default()
    } 

    pub fn is_key_down(&self, key: winit::event::VirtualKeyCode) -> bool {
        self.down.contains(&key)
    }
    
    pub fn is_key_pressed(&self, key: winit::event::VirtualKeyCode) -> bool {
        self.pressed.contains(&key)
    }

    pub fn on_event(&mut self, event: &winit::event::Event<()>) {
        if let winit::event::Event::DeviceEvent { event, .. } = event {
            if let winit::event::DeviceEvent::Key(key) = event {
                if let Some(keycode) = key.virtual_keycode {
                    match key.state {
                        winit::event::ElementState::Pressed => { self.down.insert(keycode); self.pressed.insert(keycode) },
                        winit::event::ElementState::Released => self.down.remove(&keycode)
                    }; 
                }
            } 
        } 

        if let winit::event::Event::WindowEvent { event, .. } = event {
            if let winit::event::WindowEvent::Focused(false) = event {
                self.down.clear();
                self.pressed.clear();
            }
        }
    }

    pub fn frame_finished(&mut self) {
        self.pressed.clear();
    }
}

#[derive(Default)]
pub struct Mouse {
    pub delta: Vec2,
    pub position: Vec2,
    down: HashSet<winit::event::MouseButton>,
    pressed: HashSet<winit::event::MouseButton>
}

impl Mouse {
    pub fn new() -> Self {
        Self::default()
    } 

    pub fn is_button_down(&self, key: winit::event::MouseButton) -> bool {
        self.down.contains(&key)
    }
    
    pub fn is_button_pressed(&self, key: winit::event::MouseButton) -> bool {
        self.pressed.contains(&key)
    }

    pub fn on_event(&mut self, event: &winit::event::Event<()>) {
        if let winit::event::Event::WindowEvent { event, .. } = event {
            if let winit::event::WindowEvent::MouseInput { state, button, .. } = event {
                match state {
                    winit::event::ElementState::Pressed => { self.down.insert(*button); self.pressed.insert(*button) },
                    winit::event::ElementState::Released => self.down.remove(button)
                };
            }

            if let winit::event::WindowEvent::CursorMoved { position, ..} = event {
                self.position = Vec2::new(position.x as f32, position.y as f32);
            }
        }

        if let winit::event::Event::DeviceEvent { event, .. } = event {
            if let winit::event::DeviceEvent::MouseMotion { delta } = event {
                self.delta.x = delta.0 as f32;
                self.delta.y = delta.1 as f32;
            }
        }
    }

    pub fn frame_finished(&mut self) {
        self.pressed.clear();
        self.delta = Vec2::ZERO;
    }
}
