use std::{
    collections::HashMap,
    f32::consts::PI,
    ffi::CString,
    io,
    net::{SocketAddr, UdpSocket},
    rc::Rc,
    time::Instant,
};

use glam::Vec4Swizzles;
use rand::Rng;

use crate::{
    camera::Camera,
    game_object::{GameObject, TransformComponent},
    systems::{PointLightSystem, SimpleRenderSystem},
    vulkan::{
        descriptor_set::{DescriptorPool, DescriptorSetLayout, DescriptorSetWriter},
        egui::EGuiIntegration,
        Buffer, Device, Model, RenderError, Renderer, MAX_FRAMES_IN_FLIGHT,
    },
    window::{Window, WindowSettings},
    FrameInfo, GlobalUbo, Input, KeyboardMovementController, PointLight, MAX_LIGHTS, network::Network,
};

pub struct App {
    window: Window,
    device: Rc<Device>,

    renderer: Renderer,

    egui_integration: EGuiIntegration,

    global_pool: Rc<DescriptorPool>,
    global_set_layout: Rc<DescriptorSetLayout>,

    ubo_buffers: Vec<Buffer<GlobalUbo>>,

    global_descriptor_sets: Vec<ash::vk::DescriptorSet>,

    simple_render_system: SimpleRenderSystem,
    point_light_system: PointLightSystem,

    viewer_object: GameObject,
    camera_controller: KeyboardMovementController,

    game_objects: HashMap<u8, GameObject>,

    network: Network,
}

impl App {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> anyhow::Result<Self, AppError> {
        let window = Window::new(&event_loop, WindowSettings::default());

        // window.set_cursor_icon(winit::window::CursorIcon::Grab);
        // window.set_cursor_position(glam::Vec2::new(200.0, 200.0));

        let device = Device::new(
            CString::new("test").unwrap(),
            CString::new("test").unwrap(),
            window.inner(),
        )?;

        let renderer = Renderer::new(device.clone(), &window)?;

        let egui_integration = EGuiIntegration::new(
            &window,
            device.clone(),
            &renderer.swapchain,
            renderer.swapchain.swapchain_image_format,
        )?;

        let global_pool = unsafe {
            DescriptorPool::new(device.clone())
                .max_sets(MAX_FRAMES_IN_FLIGHT as u32)
                .pool_size(
                    ash::vk::DescriptorType::UNIFORM_BUFFER,
                    MAX_FRAMES_IN_FLIGHT as u32,
                )
                .build()?
        };

        let global_set_layout = unsafe {
            DescriptorSetLayout::new(renderer.device.clone())
                .add_binding(
                    0,
                    ash::vk::DescriptorType::UNIFORM_BUFFER,
                    ash::vk::ShaderStageFlags::VERTEX | ash::vk::ShaderStageFlags::FRAGMENT,
                    1,
                )
                .build()?
        };

        let mut ubo_buffers = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            let mut buffer = Buffer::new(
                renderer.device.clone(),
                1,
                ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
                ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
            )?;

            unsafe {
                buffer.map(0)?;
            }

            ubo_buffers.push(buffer);
        }

        let mut global_descriptor_sets = Vec::with_capacity(MAX_FRAMES_IN_FLIGHT);
        for i in 0..MAX_FRAMES_IN_FLIGHT {
            let buffer_info = ubo_buffers[i].descriptor_info();
            let set = unsafe {
                DescriptorSetWriter::new(global_set_layout.clone(), global_pool.clone())
                    .write_to_buffer(0, &[buffer_info])
                    .build()
                    .unwrap()
            };

            global_descriptor_sets.push(set);
        }

        let mut viewer_object = GameObject::new(None, None, None);
        viewer_object.transform.translation.z = 2.5;

        let camera_controller = KeyboardMovementController::new(None, None);

        let game_objects = Self::load_game_objects(device.clone())?;

        let simple_render_system = SimpleRenderSystem::new(
            device.clone(),
            &renderer.swapchain.render_pass,
            &[global_set_layout.inner()],
        )?;

        let point_light_system = PointLightSystem::new(
            device.clone(),
            &renderer.swapchain.render_pass,
            &[global_set_layout.inner()],
        )?;

        Ok(Self {
            window,
            device,

            renderer,

            egui_integration,

            global_pool,
            global_set_layout,

            ubo_buffers,

            global_descriptor_sets,

            simple_render_system,
            point_light_system,

            viewer_object,
            camera_controller,

            game_objects,

            network: Network::new(),
        })
    }

    pub fn run(
        mut app: App,
        event_loop: winit::event_loop::EventLoop<()>,
    ) -> anyhow::Result<(), AppError> {
        let mut current_time = Instant::now();

        let mut input = Input::new();

        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;

            match event {
                winit::event::Event::WindowEvent { event, .. } => {
                    input.update(&event);
                    app.egui_integration.on_event(&event);

                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                            *control_flow = winit::event_loop::ControlFlow::Exit;
                            return;
                        }
                        winit::event::WindowEvent::Resized(size) => {
                            if size != app.window.inner().inner_size() {
                                app.resize().unwrap(); // TODO: fix unwrap?
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
                    app.update().unwrap(); // TODO: fix unwrap?

                    app.window.request_redraw();
                }
                winit::event::Event::RedrawRequested(_) => {
                    let frame_time = current_time.elapsed().as_secs_f32();
                    current_time = Instant::now();

                    app.draw(&input, frame_time).unwrap(); // TODO: fix unwrap?
                }
                _ => {}
            }
        });
    }

    pub fn update(&mut self) -> anyhow::Result<(), AppError> {
        if let Some(server_message) = self.network.update()? {
            match server_message {
                common::ServerMessage::ClientJoining => {
                    println!("a new client joined the server!");

                    self.spawn_game_object()?;
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn spawn_game_object(&mut self) -> anyhow::Result<(), AppError> {
        // shouldn't be loading new model each time, temporary
        let flat_vase_model = Model::from_file(
            self.device.clone(),
            "client/models/flat_vase.obj", // needs fixing for release mode
        )?;

        let mut rng = rand::thread_rng();

        // random is temporary
        let flat_vase = GameObject::new(
            Some(flat_vase_model),
            None,
            Some(TransformComponent {
                translation: glam::vec3(
                    rng.gen_range(-10..10) as f32,
                    0.0,
                    rng.gen_range(-10..10) as f32,
                ),
                scale: glam::Vec3::ONE,
                rotation: glam::Vec3::ZERO,
            }),
        );

        self.game_objects.insert(flat_vase.id, flat_vase);

        Ok(())
    }

    pub fn draw(&mut self, input: &Input, frame_time: f32) -> anyhow::Result<(), AppError> {
        self.camera_controller
            .move_in_plane_xz(input, frame_time, &mut self.viewer_object);

        let aspect = self.renderer.swapchain.extent_aspect_ratio();

        let camera = Camera::new()
            .set_perspective_projection(50_f32.to_radians(), aspect, 0.1, 100.0)
            .set_view_xyz(
                self.viewer_object.transform.translation,
                self.viewer_object.transform.rotation,
            )
            .build();

        let extent = Renderer::get_window_extent(&self.window);

        if extent.width == 0 || extent.height == 0 {
            return Ok(());
        }

        match self.renderer.begin_frame(&self.window)? {
            Some(command_buffer) => {
                let frame_index = self.renderer.frame_index();

                let mut frame_info = FrameInfo {
                    frame_index,
                    frame_time,
                    command_buffer,
                    camera,
                    global_descriptor_set: self.global_descriptor_sets[frame_index],
                    game_objects: &mut self.game_objects,
                };

                // update

                let mut ubo = GlobalUbo {
                    projection: frame_info.camera.projection_matrix,
                    view: frame_info.camera.view_matrix,
                    inverse_view: frame_info.camera.inverse_view_matrix,
                    ambient_light_color: glam::vec4(1.0, 1.0, 1.0, 0.02),
                    point_lights: [PointLight {
                        position: Default::default(),
                        color: Default::default(),
                    }; MAX_LIGHTS],
                    num_lights: 0,
                };

                self.point_light_system.update(&mut frame_info, &mut ubo);

                unsafe {
                    self.ubo_buffers[frame_index].write_to_buffer(&[ubo]);
                    self.ubo_buffers[frame_index].flush()?;
                }

                // render

                self.renderer.begin_swapchain_render_pass(command_buffer);

                // order here matters
                self.simple_render_system
                    .render_game_objects(&mut frame_info);
                self.point_light_system.render(&mut frame_info);

                self.renderer.end_swapchain_render_pass(command_buffer);

                // egui
                self.egui_integration.begin_frame(&self.window);

                egui::TopBottomPanel::top("top_panel").show(
                    &self.egui_integration.egui_ctx,
                    |ui| {
                        egui::menu::bar(ui, |ui| {
                            ui.menu_button("File", |ui| if ui.button("Test").clicked() {});
                        });
                    },
                );

                egui::SidePanel::left("my_side_panel").show(
                    &self.egui_integration.egui_ctx.clone(),
                    |ui| {
                        ui.heading("Wanhope");
                        ui.separator();
                        if !self.network.connected {
                            if ui.button("Connect").clicked() {
                                self.network.connect().unwrap(); // TODO: fix unwrap?
                            };
                        } else {
                            ui.label(format!("Connected to {}", self.network.server_ip().unwrap())); // unwrap galore
                        }
                        ui.separator();
                    },
                );

                self.egui_integration.end_frame(&mut self.window);

                self.egui_integration.paint(
                    command_buffer,
                    self.renderer.image_index(),
                )?;

                self.renderer.end_frame()?;
            }
            None => {}
        }

        Ok(())
    }

    pub fn resize(&mut self) -> anyhow::Result<(), AppError> {
        self.renderer.recreate_swapchain(&self.window)?;

        self.egui_integration.update_swapchain(
            &self.window,
            &self.renderer.swapchain,
            self.renderer.swapchain.swapchain_image_format,
        )?;

        Ok(())
    }

    fn load_game_objects(device: Rc<Device>) -> anyhow::Result<HashMap<u8, GameObject>, AppError> {
        let mut game_objects = HashMap::new();

        let floor_model = Model::from_file(
            device.clone(),
            "client/models/quad.obj", // needs fixing for release mode
        )?;

        let floor = GameObject::new(
            Some(floor_model),
            None,
            Some(TransformComponent {
                translation: glam::vec3(0.0, 0.5, 0.0),
                scale: glam::vec3(3.0, 1.0, 3.0),
                rotation: glam::Vec3::ZERO,
            }),
        );

        game_objects.insert(floor.id, floor);

        let flat_vase_model = Model::from_file(
            device.clone(),
            "client/models/flat_vase.obj", // needs fixing for release mode
        )?;

        let flat_vase = GameObject::new(
            Some(flat_vase_model),
            None,
            Some(TransformComponent {
                translation: glam::vec3(0.0, 0.5, 0.0),
                scale: glam::Vec3::ONE,
                rotation: glam::Vec3::ZERO,
            }),
        );

        game_objects.insert(flat_vase.id, flat_vase);

        let smooth_vase_model = Model::from_file(
            device.clone(),
            "client/models/smooth_vase.obj", // needs fixing for release mode
        )?;

        let smooth_vase = GameObject::new(
            Some(smooth_vase_model),
            None,
            Some(TransformComponent {
                translation: glam::vec3(0.5, 0.5, 0.0),
                scale: glam::Vec3::ONE,
                rotation: glam::Vec3::ZERO,
            }),
        );

        game_objects.insert(smooth_vase.id, smooth_vase);

        let light_colors = vec![
            glam::vec3(1.0, 0.1, 0.1),
            glam::vec3(0.1, 0.1, 1.0),
            glam::vec3(0.1, 1.0, 0.1),
            glam::vec3(1.0, 1.0, 0.1),
            glam::vec3(0.1, 1.0, 1.0),
            glam::vec3(1.0, 1.0, 1.0),
        ];

        for (i, color) in light_colors.iter().enumerate() {
            let mut point_light = GameObject::make_point_light(0.2, 0.1, *color);

            let rotate_light = glam::Mat4::from_axis_angle(
                glam::vec3(0.0, -1.0, 0.0),
                i as f32 * (PI * 2.0) / light_colors.len() as f32,
            );

            point_light.transform.translation =
                (rotate_light * glam::vec4(-1.0, -1.0, -1.0, 1.0)).xyz();

            game_objects.insert(point_light.id, point_light);
        }

        Ok(game_objects)
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("")]
    RenderError(#[from] RenderError),
    #[error("")]
    NetworkError(#[from] io::Error),
}
