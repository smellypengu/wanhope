use std::{ffi::CString, rc::Rc, time::Instant, net::{UdpSocket, SocketAddr}};

use rand::Rng;

use crate::{window::{Window, WindowSettings}, vulkan::{Device, Model, Renderer, Buffer, MAX_FRAMES_IN_FLIGHT, descriptor_set::{DescriptorPool, DescriptorSetLayout, DescriptorSetWriter}}, game_object::{GameObject, TransformComponent}, camera::Camera, KeyboardMovementController, SimpleRenderSystem, Input, FrameInfo};

#[derive(PartialEq)]
#[repr(C)]
pub struct GlobalUbo {
    pub projection_view: glam::Mat4,
    pub light_direction: glam::Vec3,
}

pub struct App {
    window: Window,
    device: Rc<Device>,

    renderer: Renderer,

    global_pool: Rc<DescriptorPool>,
    global_set_layout: Rc<DescriptorSetLayout>,

    ubo_buffers: Vec<Buffer<GlobalUbo>>,

    global_descriptor_sets: Vec<ash::vk::DescriptorSet>,

    simple_render_system: SimpleRenderSystem,

    viewer_object: GameObject,
    camera_controller: KeyboardMovementController,

    game_objects: Vec<GameObject>,

    socket: Option<UdpSocket>,
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

        let renderer = Renderer::new(
            device.clone(),
            &window,
        ).unwrap();

        let global_pool = unsafe {
            DescriptorPool::new(device.clone())
                .max_sets(MAX_FRAMES_IN_FLIGHT as u32)
                .pool_size(ash::vk::DescriptorType::UNIFORM_BUFFER, MAX_FRAMES_IN_FLIGHT as u32)
                .build().unwrap()
        };

        let global_set_layout = unsafe {
            DescriptorSetLayout::new(renderer.device.clone())
            .add_binding(0, ash::vk::DescriptorType::UNIFORM_BUFFER, ash::vk::ShaderStageFlags::ALL_GRAPHICS, 1)
            .build().unwrap()
        };

        let mut ubo_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let mut buffer = Buffer::new(
                renderer.device.clone(),
                1,
                ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
            ).unwrap();

            unsafe {
                buffer.map(0).unwrap();
            }

            ubo_buffers.push(buffer);
        }

        let mut global_descriptor_sets = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let buffer_info = ubo_buffers[i].descriptor_info();
            let set = unsafe {
                DescriptorSetWriter::new(global_set_layout.clone(), global_pool.clone())
                .write_to_buffer(0, &[buffer_info])
                .build().unwrap()
            };

            global_descriptor_sets.push(set);
        }

        let viewer_object = GameObject::new(None, None, None);
        let camera_controller = KeyboardMovementController::new(None, None);

        let game_objects = Self::load_game_objects(device.clone());

        let simple_render_system = SimpleRenderSystem::new(
            device.clone(),
            &renderer.swapchain.render_pass,
            &[global_set_layout.inner()],
        ).unwrap();

        Self {
            window,
            device,

            renderer,

            global_pool,
            global_set_layout,
            
            ubo_buffers,

            global_descriptor_sets,

            simple_render_system,

            viewer_object,
            camera_controller,

            game_objects,

            socket: None,
        }
    }

    pub fn run(mut app: App, event_loop: winit::event_loop::EventLoop<()>) {
        let mut current_time = Instant::now();

        let mut input = Input::new();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;
    
            match event {
                winit::event::Event::WindowEvent { event, .. } => {
                    input.update(&event);

                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                            *control_flow = winit::event_loop::ControlFlow::Exit;
                            return;
                        }
                        winit::event::WindowEvent::Resized(size) => {
                            if size != app.window.inner().inner_size() {
                                app.resize();
                            }
                        }
                        winit::event::WindowEvent::KeyboardInput { input, .. } => {
                            match input.virtual_keycode {
                                Some(winit::event::VirtualKeyCode::C) => {
                                    app.connect();
                                }
                                _ => {}
                            }
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
                        _ => {}
                    }
                }
                winit::event::Event::MainEventsCleared => {
                    app.update();

                    app.window.request_redraw();
                }
                winit::event::Event::RedrawRequested(_) => {
                    let frame_time = current_time.elapsed().as_secs_f32();
                    current_time = Instant::now();

                    app.draw(&input, frame_time);
                }
                _ => {}
            }
        });
    }

    pub fn connect(&mut self) {
        if self.socket.is_none() {
            let remote_addr: SocketAddr = "127.0.0.1:8080".parse().unwrap();

            let local_addr: SocketAddr = if remote_addr.is_ipv4() {
                "0.0.0.0:0"
            } else {
                "[::]:0"
            }
            .parse().unwrap();

            self.socket = Some(UdpSocket::bind(local_addr).unwrap());

            // register as user in server

            let socket = self.socket.as_ref().unwrap();            
            socket.connect(remote_addr).unwrap();
            
            socket.send(&[common::ClientMessage::Join as u8]).unwrap();
            
            let mut response = vec![0u8; 2];
            let len = socket.recv(&mut response).unwrap();

            let join_result = common::ServerMessage::try_from(response[0]).unwrap();

            match join_result {
                common::ServerMessage::JoinResult => {
                    if len > 1 as usize {
                        println!("user id: {}", response[1]);
                    } else {
                        println!("server did not let us in");
                    }
                }
                _ => {}
            }
        }
    }

    pub fn update(&mut self) {
        match &self.socket {
            Some(socket) => {
                let mut data = vec![0u8; 1_024];

                // set to nonblocking so recv() call doesn't freeze app, probably temporary.
                socket.set_nonblocking(true).unwrap();

                match socket.recv(&mut data) {
                    Ok(len) => {
                        let server_message = common::ServerMessage::try_from(data[0]).unwrap();
        
                        match server_message {
                            common::ServerMessage::ClientJoining => {
                                println!("a new client joined the server!");

                                self.spawn_game_object();
                            }
                            _ => {}
                        }
                    }
                    Err(_) => {}
                }

                // let test_struct = common::TestStruct { x: 100, abc: "lol".to_string() };
                // let msg = common::serialize(&test_struct).unwrap();

                // socket.send(&msg).unwrap();
                // let mut data = vec![0u8; 1_000];
                // let len = socket.recv(&mut data).unwrap();

                // let deserialized: common::TestStruct = common::deserialize(&data[..len]).unwrap();

                // println!(
                //     "Received {} bytes:\n{:?}",
                //     len,
                //     deserialized,
                // );

            },
            None => {},
        }
    }

    pub fn spawn_game_object(&mut self) {
        // shouldn't be loading new model each time, temporary.
        let flat_vase_model = Model::from_file(
            self.device.clone(),
            "client/models/flat_vase.obj", // needs fixing for release mode.
        ).unwrap();

        let mut rng = rand::thread_rng();

        // random is temporary.
        let flat_vase = GameObject::new(
            Some(flat_vase_model),
            None,
            Some(TransformComponent { translation: glam::vec3(rng.gen_range(-10..10) as f32, 0.0, rng.gen_range(-10..10) as f32), scale: glam::Vec3::ONE, rotation: glam::Vec3::ZERO }),
        );

        self.game_objects.push(flat_vase);
    }

    pub fn draw(&mut self, input: &Input, frame_time: f32) {
        self.camera_controller.move_in_plane_xz(input, frame_time, &mut self.viewer_object);

        let aspect = self.renderer.swapchain.extent_aspect_ratio();

        let camera = Camera::new()
            .set_perspective_projection(50_f32.to_radians(), aspect, 0.1, 10.0)
            .set_view_xyz(self.viewer_object.transform.translation, self.viewer_object.transform.rotation)
            .build();

        let extent = Renderer::get_window_extent(&self.window);

        if extent.width == 0 || extent.height == 0 {
            return;
        }

        match self.renderer.begin_frame(&self.window).unwrap() {
            Some(command_buffer) => {
                let frame_index = self.renderer.frame_index();

                let frame_info = FrameInfo {
                    frame_index,
                    frame_time,
                    command_buffer,
                    camera,
                    global_descriptor_set: self.global_descriptor_sets[frame_index],
                };

                //update

                let ubo = GlobalUbo {
                    projection_view: frame_info.camera.projection_matrix * frame_info.camera.view_matrix,
                    light_direction: glam::vec3(1.0, -3.0, 1.0).normalize(),
                };

                unsafe {
                    self.ubo_buffers[frame_index].write_to_buffer(&[ubo]);
                    self.ubo_buffers[frame_index].flush().unwrap();
                }

                // render

                self.renderer.begin_swapchain_render_pass(command_buffer);

                self.simple_render_system.render_game_objects(frame_info, &mut self.game_objects);

                self.renderer.end_swapchain_render_pass(command_buffer);

                self.renderer.end_frame().unwrap();
            },
            None => { },
        }
    }

    pub fn resize(&mut self) {
        self.renderer.recreate_swapchain(&self.window).unwrap();
    }

    fn load_game_objects(
        device: Rc<Device>,
    ) -> Vec<GameObject> {
        let flat_vase_model = Model::from_file(
            device.clone(),
            "client/models/flat_vase.obj", // needs fixing for release mode.
        ).unwrap();

        let flat_vase = GameObject::new(
            Some(flat_vase_model),
            None,
            Some(TransformComponent { translation: glam::vec3(0.0, 0.0, -2.5), scale: glam::Vec3::ONE, rotation: glam::Vec3::ZERO }),
        );

        let smooth_vase_model = Model::from_file(
            device.clone(),
            "client/models/smooth_vase.obj", // needs fixing for release mode.
        ).unwrap();

        let smooth_vase = GameObject::new(
            Some(smooth_vase_model),
            None,
            Some(TransformComponent { translation: glam::vec3(0.5, 0.0, -2.5), scale: glam::Vec3::ONE, rotation: glam::Vec3::ZERO }),
        );

        vec![flat_vase, smooth_vase]
    }
}