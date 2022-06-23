use std::{collections::HashMap, f32::consts::PI, ffi::CString, rc::Rc, time::Instant};

use glam::Vec4Swizzles;

use crate::{
    egui::{ui, EGui},
    game_object::{GameObject, TransformComponent},
    graphics::{
        systems::{PointLightSystem, SimpleRenderSystem},
        vulkan::{
            descriptor_set::{DescriptorPool, DescriptorSetLayout, DescriptorSetWriter},
            Buffer, Device, Image, Model, Renderer, Vertex, MAX_FRAMES_IN_FLIGHT,
        },
        Camera, FrameInfo, GlobalUbo, PointLight, RenderError, TileAtlas, Window, WindowSettings,
        MAX_LIGHTS,
    },
    network::{Network, NetworkError},
    Input, KeyboardMovementController, ModelAsset, TextureAsset,
};

pub struct App {
    window: Window,
    device: Rc<Device>,

    renderer: Renderer,

    egui: EGui,
    egui_hovered: bool,

    global_pool: Rc<DescriptorPool>,
    global_set_layout: Rc<DescriptorSetLayout>,
    global_descriptor_sets: Vec<ash::vk::DescriptorSet>,

    ubo_buffers: Vec<Buffer<GlobalUbo>>,

    image_set_layout: Rc<DescriptorSetLayout>,
    image_descriptor_set: ash::vk::DescriptorSet,

    tile_atlas: TileAtlas,
    tile_atlas_image: Rc<Image>,

    simple_render_system: SimpleRenderSystem,
    point_light_system: PointLightSystem,

    camera: Option<Camera>,
    camera_controller: KeyboardMovementController,
    viewer_object: GameObject,

    game_objects: HashMap<u8, GameObject>,

    select_id: u8,

    network: Network,

    players: Vec<Option<common::Player>>,

    world: Option<common::world::World>,

    chunk_position_to_ids: HashMap<common::Position, u8>,
}

impl App {
    pub fn new(event_loop: &winit::event_loop::EventLoop<()>) -> anyhow::Result<Self, AppError> {
        let window = Window::new(&event_loop, WindowSettings::default());

        // window.set_cursor_icon(winit::window::CursorIcon::Grab);
        // window.set_cursor_position(glam::Vec2::new(200.0, 200.0));

        let device = unsafe {
            Device::new(
                CString::new("wanhope").unwrap(),
                CString::new("wanhope").unwrap(),
                window.inner(),
            )?
        };

        let renderer = Renderer::new(device.clone(), &window)?;

        let egui = EGui::new(&window, &renderer)?;

        let global_pool = unsafe {
            DescriptorPool::new(device.clone())
                .max_sets(1000 as u32)
                .pool_size(
                    ash::vk::DescriptorType::UNIFORM_BUFFER,
                    MAX_FRAMES_IN_FLIGHT as u32,
                )
                .pool_size(
                    ash::vk::DescriptorType::SAMPLED_IMAGE,
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
            let mut buffer = unsafe {
                Buffer::new(
                    renderer.device.clone(),
                    1,
                    ash::vk::BufferUsageFlags::UNIFORM_BUFFER,
                    ash::vk::MemoryPropertyFlags::HOST_VISIBLE,
                )?
            };

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

        let image_set_layout = unsafe {
            DescriptorSetLayout::new(device.clone())
                .add_binding(
                    0,
                    ash::vk::DescriptorType::COMBINED_IMAGE_SAMPLER,
                    ash::vk::ShaderStageFlags::ALL_GRAPHICS,
                    1,
                )
                .build()?
        };

        let mut tile_atlas = TileAtlas::new(4, 32)?;

        for path in TextureAsset::iter() {
            let asset = TextureAsset::get(&path).unwrap();

            tile_atlas.add_texture(asset)?;
        }

        let tile_atlas_image = tile_atlas.build(device.clone())?;

        let image_descriptor_set = unsafe {
            DescriptorSetWriter::new(image_set_layout.clone(), global_pool.clone())
                .write_image(0, &[tile_atlas_image.image_info])
                .build()
                .unwrap()
        };

        let camera_controller = KeyboardMovementController::new(None, None);

        let mut viewer_object = GameObject::new(None, None, None);
        viewer_object.transform.translation.z = 2.5;

        let (game_objects, select_id) = Self::load_game_objects(device.clone())?;

        let simple_render_system = SimpleRenderSystem::new(
            device.clone(),
            &renderer.swapchain.render_pass,
            &[global_set_layout.inner(), image_set_layout.inner()],
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

            egui,
            egui_hovered: false,

            global_pool,
            global_set_layout,
            global_descriptor_sets,

            image_set_layout,
            image_descriptor_set,

            tile_atlas,
            tile_atlas_image,

            ubo_buffers,

            simple_render_system,
            point_light_system,

            camera: None,
            camera_controller,
            viewer_object,

            game_objects,

            select_id,

            network: Network::new(),

            players: Vec::new(),

            world: None,

            chunk_position_to_ids: HashMap::new(),
        })
    }

    pub fn run(
        mut app: App,
        event_loop: winit::event_loop::EventLoop<()>,
    ) -> anyhow::Result<(), AppError> {
        let mut current_time = Instant::now();
        let mut frame_time = 0.0;

        let mut input = Input::new();

        let mut recreate_swapchain = false;

        event_loop.run(move |event, _, control_flow| {
            *control_flow = winit::event_loop::ControlFlow::Poll;

            match event {
                winit::event::Event::WindowEvent { event, .. } => {
                    input.update(&event);
                    app.egui.on_event(&event);

                    match event {
                        winit::event::WindowEvent::CloseRequested => {
                            app.network.leave().unwrap(); // TODO: fix unwrap?

                            *control_flow = winit::event_loop::ControlFlow::Exit;
                            return;
                        }
                        winit::event::WindowEvent::Resized(size) => {
                            if size != app.window.inner().inner_size() {
                                recreate_swapchain = true;
                            }
                        }
                        winit::event::WindowEvent::CursorMoved { position, .. } => {
                            let physical_position = glam::DVec2::new(position.x, position.y);
                            app.window.physical_cursor_position = Some(physical_position);
                        }
                        winit::event::WindowEvent::CursorLeft { .. } => {
                            app.window.physical_cursor_position = None;
                        }
                        winit::event::WindowEvent::MouseInput { state, button, .. } => {
                            app.mouse_input(state, button).unwrap(); // TODO: fix unwrap?
                        }
                        _ => {}
                    }
                }
                winit::event::Event::MainEventsCleared => {
                    frame_time = current_time.elapsed().as_secs_f32();
                    current_time = Instant::now();

                    app.update(&input, frame_time).unwrap(); // TODO: fix unwrap?

                    app.window.request_redraw();
                }
                winit::event::Event::RedrawRequested(_) => {
                    app.draw(frame_time, recreate_swapchain).unwrap(); // TODO: fix unwrap?

                    recreate_swapchain = false;
                }
                _ => {}
            }
        });
    }

    pub fn update(&mut self, input: &Input, frame_time: f32) -> anyhow::Result<(), AppError> {
        self.camera_controller
            .move_in_plane_xz(input, frame_time, &mut self.viewer_object);

        let aspect = self.renderer.swapchain.extent_aspect_ratio();

        self.camera = Some(
            Camera::new()
                .set_perspective_projection(50_f32.to_radians(), aspect, 0.1, 100.0)
                .set_view_xyz(
                    self.viewer_object.transform.translation,
                    self.viewer_object.transform.rotation,
                )
                .build(),
        );

        if let (Some(server_message), payload) = self.network.update()? {
            match server_message {
                common::ServerPacket::ClientJoin => {
                    log::info!("a client joined the server!");

                    self.players =
                        bincode::decode_from_slice(&payload, bincode::config::standard())
                            .unwrap()
                            .0;
                }
                common::ServerPacket::ClientLeave => {
                    log::info!("a client left the server!");

                    self.players =
                        bincode::decode_from_slice(&payload, bincode::config::standard())
                            .unwrap()
                            .0;
                }
                common::ServerPacket::Chat => {
                    let message = std::str::from_utf8(&payload)
                        .unwrap()
                        .trim_matches(char::from(0))
                        .to_string();

                    if let Some(chat) = self.egui.get_mut::<ui::Chat>("Chat") {
                        chat.messages.push(message);
                    }
                }
                common::ServerPacket::ChunkModified => {
                    let chunk: common::world::Chunk =
                        bincode::serde::decode_from_slice(&payload, bincode::config::standard())
                            .unwrap()
                            .0;

                    // should always be some
                    if let Some(world) = &mut self.world {
                        world
                            .chunks
                            .get_mut((chunk.position.x, chunk.position.y))
                            .unwrap()
                            .tiles = chunk.tiles.clone(); // hmmmm

                        log::info!("recreating chunk game object");

                        if let Some(id) = self.chunk_position_to_ids.remove(&chunk.position) {
                            self.game_objects.remove(&id);
                        }

                        let id = self.create_chunk_game_object(&chunk)?;

                        self.chunk_position_to_ids.insert(chunk.position, id);
                    }
                }
                _ => {}
            }
        }

        Ok(())
    }

    pub fn mouse_input(
        &mut self,
        state: winit::event::ElementState,
        button: winit::event::MouseButton,
    ) -> anyhow::Result<(), AppError> {
        if let Some(world) = &self.world {
            if self.egui_hovered
                || state != winit::event::ElementState::Pressed
                || button != winit::event::MouseButton::Left
            {
                return Ok(());
            }

            if let Some(cursor_position) = self.window.cursor_position() {
                if let Some(ray) = crate::graphics::Ray::from_screenspace(
                    cursor_position,
                    glam::vec2(
                        self.window.inner().inner_size().width as f32,
                        self.window.inner().inner_size().height as f32,
                    ),
                    self.camera.as_ref().unwrap(),
                ) {
                    let plane = crate::graphics::Plane {
                        center: glam::Vec3::ZERO,
                        normal: glam::Vec3::Y,
                    };

                    if let Some(distance) = plane.intersect(&ray) {
                        let point = ray.origin + ray.dir * distance;

                        let position = (glam::vec2((point.x - 0.5) / 10.0, (point.z + 0.5) / 10.0)
                            * 10.0)
                            .round();

                        let p = position - glam::Vec2::Y;

                        if p.x >= 0.0
                            && p.y >= 0.0
                            && p.x < (world.width * common::world::CHUNK_SIZE) as f32
                            && p.y < (world.height * common::world::CHUNK_SIZE) as f32
                        {
                            self.network.send_client_world_click(p.abs())?;

                            self.game_objects
                                .get_mut(&self.select_id)
                                .unwrap()
                                .transform
                                .translation = glam::vec3(position.x + 0.5, 0.0, position.y - 0.5);
                        }
                    }
                }
            }
        }

        Ok(())
    }

    pub fn draw(
        &mut self,
        frame_time: f32,
        recreate_swapchain: bool,
    ) -> anyhow::Result<(), AppError> {
        let extent = Renderer::get_window_extent(&self.window);

        if extent.width == 0 || extent.height == 0 {
            return Ok(());
        }

        if recreate_swapchain {
            self.resize()?;
        }

        match unsafe { self.renderer.begin_frame(&self.window)? } {
            Some(command_buffer) => {
                let frame_index = self.renderer.frame_index();

                let mut frame_info = FrameInfo {
                    frame_index,
                    frame_time,
                    command_buffer,
                    camera: self.camera.as_ref().unwrap(),
                    global_descriptor_set: self.global_descriptor_sets[frame_index],
                    image_descriptor_set: self.image_descriptor_set,
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

                    // render
                    self.renderer.begin_swapchain_render_pass(command_buffer);

                    // order here matters
                    self.simple_render_system
                        .render_game_objects(&mut frame_info);
                    self.point_light_system.render(&mut frame_info);

                    self.renderer.end_swapchain_render_pass(command_buffer);

                    let r = self.egui.render(
                        &self.window,
                        &self.renderer,
                        command_buffer,
                        &mut self.network,
                        &self.players,
                        &self.world,
                    )?;

                    self.egui_hovered = r.0;

                    if let Some(world) = r.1 {
                        self.create_chunk_game_objects(&world)?;

                        self.world = Some(world);
                    }

                    self.renderer.end_frame(&self.window)?;
                }
            }
            None => {}
        }

        Ok(())
    }

    pub fn resize(&mut self) -> anyhow::Result<(), AppError> {
        unsafe {
            self.renderer.recreate_swapchain(&self.window)?;

            self.egui.resize(&self.window, &self.renderer)?;
        }

        Ok(())
    }

    fn create_chunk_game_objects(
        &mut self,
        world: &common::world::World,
    ) -> anyhow::Result<(), AppError> {
        for x in 0..world.width {
            for y in 0..world.height {
                let chunk = world.chunks.get((x, y)).unwrap();

                let id = self.create_chunk_game_object(chunk)?;

                self.chunk_position_to_ids.insert(chunk.position, id);
            }
        }

        Ok(())
    }

    fn create_chunk_game_object(
        &mut self,
        chunk: &common::world::Chunk,
    ) -> anyhow::Result<u8, AppError> {
        let mut vertices: Vec<Vertex> = Vec::new();

        for chunk_x in 0..common::world::CHUNK_SIZE {
            for chunk_y in 0..common::world::CHUNK_SIZE {
                let tile = chunk.tiles.get((chunk_x, chunk_y)).unwrap();

                let size = (1.0 / 16.0)
                    - (1.0 / (self.tile_atlas.size * self.tile_atlas.tile_size) as f32) * 2.0;

                let offset_x = match tile.ty {
                    common::world::TileType::Grass => 0.0,
                    common::world::TileType::Sand => {
                        32.0 * ((1.0 / 16.0)
                            + (1.0 / (self.tile_atlas.size * self.tile_atlas.tile_size) as f32))
                    }
                };

                let offset_y = 0.0;

                vertices.push(Vertex {
                    position: glam::vec3(chunk_x as f32, 0.0, chunk_y as f32),
                    color: glam::vec3(1.0, 1.0, 1.0),
                    normal: glam::vec3(0.0, 0.0, 0.0),
                    uv: glam::vec2(offset_x, offset_y),
                });

                vertices.push(Vertex {
                    position: glam::vec3(chunk_x as f32 + 1.0, 0.0, chunk_y as f32),
                    color: glam::vec3(1.0, 1.0, 1.0),
                    normal: glam::vec3(0.0, 0.0, 0.0),
                    uv: glam::vec2(offset_x + size, offset_y),
                });

                vertices.push(Vertex {
                    position: glam::vec3(chunk_x as f32, 0.0, chunk_y as f32 + 1.0),
                    color: glam::vec3(1.0, 1.0, 1.0),
                    normal: glam::vec3(0.0, 0.0, 0.0),
                    uv: glam::vec2(offset_x, offset_y + size),
                });

                vertices.push(Vertex {
                    position: glam::vec3(chunk_x as f32 + 1.0, 0.0, chunk_y as f32),
                    color: glam::vec3(1.0, 1.0, 1.0),
                    normal: glam::vec3(0.0, 0.0, 0.0),
                    uv: glam::vec2(offset_x + size, offset_y),
                });

                vertices.push(Vertex {
                    position: glam::vec3(chunk_x as f32, 0.0, chunk_y as f32 + 1.0),
                    color: glam::vec3(1.0, 1.0, 1.0),
                    normal: glam::vec3(0.0, 0.0, 0.0),
                    uv: glam::vec2(offset_x, offset_y + size),
                });

                vertices.push(Vertex {
                    position: glam::vec3(chunk_x as f32 + 1.0, 0.0, chunk_y as f32 + 1.0),
                    color: glam::vec3(1.0, 1.0, 1.0),
                    normal: glam::vec3(0.0, 0.0, 0.0),
                    uv: glam::vec2(offset_x + size, offset_y + size),
                });
            }
        }

        let model = Model::new(self.device.clone(), &vertices, None)?;

        let obj = GameObject::new(
            Some(model),
            None,
            Some(TransformComponent {
                translation: glam::vec3(
                    (chunk.position.x * common::world::CHUNK_SIZE) as f32,
                    0.0,
                    (chunk.position.y * common::world::CHUNK_SIZE) as f32,
                ),
                scale: glam::Vec3::ONE,
                rotation: glam::Vec3::ZERO,
            }),
        );

        let id = obj.id;

        self.game_objects.insert(id, obj);

        Ok(id)
    }

    fn load_game_objects(
        device: Rc<Device>,
    ) -> anyhow::Result<(HashMap<u8, GameObject>, u8), AppError> {
        let mut game_objects = HashMap::new();

        let floor_model = Model::from_file(device.clone(), ModelAsset::get("quad.obj").unwrap())?;

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

        let flat_vase_model =
            Model::from_file(device.clone(), ModelAsset::get("flat_vase.obj").unwrap())?;

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

        let smooth_vase_model =
            Model::from_file(device.clone(), ModelAsset::get("smooth_vase.obj").unwrap())?;

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

        let select_model = Model::from_file(device.clone(), ModelAsset::get("quad.obj").unwrap())?;

        let select = GameObject::new(
            Some(select_model),
            None,
            Some(TransformComponent {
                translation: glam::vec3(0.5, 0.0, -0.5),
                scale: glam::vec3(0.5, 1.0, 0.5),
                rotation: glam::Vec3::ZERO,
            }),
        );

        let select_id = select.id;

        game_objects.insert(select_id, select);

        Ok((game_objects, select_id))
    }
}

#[derive(thiserror::Error, Debug)]
pub enum AppError {
    #[error("")]
    RenderError(#[from] RenderError),
    #[error("")]
    NetworkError(#[from] NetworkError),
}
