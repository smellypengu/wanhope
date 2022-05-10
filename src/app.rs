use std::{ffi::CString, rc::Rc};

use crate::{window::{Window, WindowSettings}, vulkan::{Device, Model, Vertex, Align16, Renderer, SimpleRenderSystem}, game_object::{GameObject, TransformComponent}};

pub struct App {
    window: Window,
    device: Rc<Device>,

    renderer: Renderer,

    game_objects: Vec<GameObject>,

    simple_render_system: SimpleRenderSystem,
}

impl App {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> Self {
        let window = Window::new(
            &event_loop,
            WindowSettings::default(),
        );

        // window.set_cursor_icon(winit::window::CursorIcon::Grab);
        // window.set_cursor_position(glam::Vec2::new(200.0, 200.0));

        let device = Device::new(
            CString::new("test").unwrap(),
            CString::new("test").unwrap(),
            window.inner()
        ).unwrap();

        let window_extent = ash::vk::Extent2D {
            width: window.inner().inner_size().width,
            height: window.inner().inner_size().height,
        };

        let game_objects = Self::load_game_objects(device.clone());

        let renderer = Renderer::new(
            device.clone(),
            &window,
        ).unwrap();

        let simple_render_system = SimpleRenderSystem::new(
            device.clone(),
            &renderer.swapchain.render_pass
        ).unwrap();

        Self {
            window,
            device,

            renderer,

            game_objects,

            simple_render_system,
        }
    }

    pub fn run(mut app: App, event_loop: winit::event_loop::EventLoop<()>) {
        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;
    
            match event {
                winit::event::Event::WindowEvent { event, .. } => {
                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                            *control_flow = winit::event_loop::ControlFlow::Exit
                        }
                        winit::event::WindowEvent::Resized(size) => {
                            app.resize();
                        }
                        winit::event::WindowEvent::CursorMoved { position, .. } => {
                            let height = app.window.inner().inner_size().height;
    
                            // move origin to bottom left
                            let y = height as f64 - position.y;
    
                            let physical_position = glam::DVec2::new(position.x, y);
                            app.window.physical_cursor_position = Some(physical_position);
                        }
                        winit::event::WindowEvent::CursorLeft { .. } => {
                            app.window.physical_cursor_position = None;
                        }
                        _ => ()
                    }
                }
                winit::event::Event::MainEventsCleared => {
                    app.window.request_redraw();
                }
                winit::event::Event::RedrawRequested(_) => {
                    app.render();
                }
                _ => ()
            }
        });
    }

    fn load_game_objects(
        device: Rc<Device>,
    ) -> Vec<GameObject> {
        let vertices = vec![
            Vertex {
                position: glam::vec2(0.0, -0.5),
                color: glam::vec3(1.0, 0.0, 0.0),
            },
            Vertex {
                position: glam::vec2(0.5, 0.5),
                color: glam::vec3(0.0, 1.0, 0.0),
            },
            Vertex {
                position: glam::vec2(-0.5, 0.5),
                color: glam::vec3(0.0, 0.0, 1.0),
            },
        ];

        let model = Model::new(
            device,
            &vertices,
            None,
        ).unwrap();

        let game_object = GameObject::new(
            Some(model),
            Some(glam::vec3(0.1, 0.8, 0.1)),
            Some(TransformComponent { translation: glam::vec2(0.2, 0.0) }),
        );

        vec![game_object]
    }

    pub fn render(&mut self) {
        match self.renderer.begin_frame(&self.window).unwrap() {
            Some(command_buffer) => {
                self.renderer.begin_swapchain_render_pass(command_buffer);

                self.simple_render_system.render_game_objects(command_buffer, &self.game_objects);

                self.renderer.end_swapchain_render_pass(command_buffer);

                self.renderer.end_frame().unwrap();
            },
            None => { },
        }
    }

    pub fn resize(&mut self) {
        self.renderer.recreate_swapchain(&self.window).unwrap();
    }
}
