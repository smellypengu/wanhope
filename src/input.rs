use std::collections::HashMap;

pub struct Input {
    keymap: HashMap<winit::event::VirtualKeyCode, bool>,
}

impl Input {
    pub fn new() -> Self {
        Self {
            keymap: HashMap::new(),
        }
    }

    pub fn key_held(&self, key: winit::event::VirtualKeyCode) -> bool {
        if let Some(value) = self.keymap.get(&key) {
            *value
        } else {
            false
        }
    }

    pub fn update_key(&mut self, input: &winit::event::KeyboardInput) {
        input.virtual_keycode.map(|keycode| {
            self.keymap.insert(
                keycode,
                match input.state {
                    winit::event::ElementState::Pressed => true,
                    _ => false,
                }
            );
        });
    }

    pub fn update(&mut self, event: &winit::event::WindowEvent) {
        match event {
            winit::event::WindowEvent::KeyboardInput { input, .. } => self.update_key(input),
            _ => (),
        }
    }
}
