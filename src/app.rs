use std::{ffi::CString, rc::Rc};

use crate::{window::{Window, WindowSettings}, vulkan::{Device, Pipeline, Swapchain, Model, Vertex}};

pub struct App {
    window: Window,
    device: Rc<Device>,
    swapchain: Swapchain,
    pipeline_layout: ash::vk::PipelineLayout,
    pipeline: Rc<Pipeline>,
    command_buffers: Vec<ash::vk::CommandBuffer>,

    model: Rc<Model>,
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

        let swapchain = Swapchain::new(
            device.clone(),
            window_extent,
            None,
        ).unwrap();

        let pipeline_layout_info = ash::vk::PipelineLayoutCreateInfo::builder();
    
        let pipeline_layout = unsafe {
            device.logical_device.create_pipeline_layout(
                &pipeline_layout_info,
                None,
            ).unwrap()
        };

        let pipeline = Pipeline::start()
            .binding_descriptions(Vertex::binding_descriptions())
            .attribute_descriptions(Vertex::attribute_descriptions())
            .build(
                device.clone(),
                "shaders/simple_shader.vert.spv",
                "shaders/simple_shader.frag.spv",
                &swapchain.render_pass,
                &pipeline_layout,
            ).unwrap();

        let model = Self::load_models(device.clone());

        let command_buffers = Self::create_command_buffers(
            &device, 
            &swapchain, 
            &pipeline,
            &model,
        );

        Self {
            window,
            device,
            swapchain,
            pipeline_layout,
            pipeline,
            command_buffers,

            model,
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

    fn load_models(
        device: Rc<Device>,
    ) -> Rc<Model> {
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

        Model::new(
            device,
            &vertices,
            None,
        ).unwrap()
    }

    // temporary
    fn create_command_buffers(
        device: &Rc<Device>,
        swapchain: &Swapchain,
        pipeline: &Rc<Pipeline>,
        model: &Rc<Model>,
    ) -> Vec<ash::vk::CommandBuffer> {
        let alloc_info = ash::vk::CommandBufferAllocateInfo::builder()
            .level(ash::vk::CommandBufferLevel::PRIMARY)
            .command_pool(device.command_pool)
            .command_buffer_count(swapchain.swapchain_images.len() as u32);

        let command_buffers = unsafe {
            device.logical_device
                .allocate_command_buffers(&alloc_info)
                .map_err(|e| log::error!("Unable to allocate command buffer: {}", e))
                .unwrap()
        };

        command_buffers
    }

    // temporary
    fn record_command_buffers(&self, image_index: usize) {
        let begin_info = ash::vk::CommandBufferBeginInfo::builder();

        unsafe {
            self.device
                .logical_device
                .begin_command_buffer(self.command_buffers[image_index], &begin_info)
                .map_err(|e| log::error!("Unable to begin command buffer: {}", e))
                .unwrap();

            let render_area = ash::vk::Rect2D {
                offset: ash::vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.swapchain_extent,
            };

            let color_clear = ash::vk::ClearValue {
                color: ash::vk::ClearColorValue {
                    float32: [0.01, 0.01, 0.01, 1.0],
                },
            };
    
            let depth_clear = ash::vk::ClearValue {
                depth_stencil: ash::vk::ClearDepthStencilValue {
                    depth: 1.0,
                    stencil: 0,
                },
            };

            let clear_values = [color_clear, depth_clear];

            let render_pass_info = ash::vk::RenderPassBeginInfo::builder()
                .render_pass(self.swapchain.render_pass)
                .framebuffer(self.swapchain.swapchain_framebuffers[image_index])
                .render_area(render_area)
                .clear_values(&clear_values);

            self.device.logical_device.cmd_begin_render_pass(
                self.command_buffers[image_index],
                &render_pass_info,
                ash::vk::SubpassContents::INLINE,
            );

            let viewport = ash::vk::Viewport {
                x: 0.0,
                y: 0.0,
                width: self.swapchain.width() as f32,
                height: self.swapchain.height() as f32,
                min_depth: 0.0,
                max_depth: 1.0,
            };

            let scissor = ash::vk::Rect2D {
                offset: ash::vk::Offset2D { x: 0, y: 0 },
                extent: self.swapchain.swapchain_extent,
            };

            self.device
                .logical_device
                .cmd_set_viewport(self.command_buffers[image_index], 0, &[viewport]);
            self.device
                .logical_device
                .cmd_set_scissor(self.command_buffers[image_index], 0, &[scissor]);

            self.pipeline.bind(self.command_buffers[image_index]);
            self.model.bind(self.command_buffers[image_index]);
            self.model.draw(&self.device.logical_device, self.command_buffers[image_index]);

            self.device.logical_device.cmd_end_render_pass(self.command_buffers[image_index]);

            self.device.logical_device.end_command_buffer(self.command_buffers[image_index]).unwrap();
        };
    }

    fn recreate_swapchain(&mut self) {
        let window_extent = ash::vk::Extent2D {
            width: self.window.inner().inner_size().width,
            height: self.window.inner().inner_size().height,
        };

        if window_extent.width == 0 || window_extent.height == 0 {
            return; // Don't do anything if the window is minimized
        }

        log::debug!("Recreating vulkan swapchain");

        unsafe {
            self.device
                .logical_device
                .device_wait_idle().unwrap()
        };

        let new_swapchain = Swapchain::new(
            self.device.clone(),
            window_extent,
            self.swapchain.swapchain_khr,
        ).unwrap();

        self.swapchain
            .compare_swap_formats(&new_swapchain)
            .map_err(|_| log::error!("Swapchain image or depth format has changed"))
            .unwrap();

        self.swapchain = new_swapchain;
    }

    pub fn render(&mut self) {
        let result = unsafe {
            self.swapchain.acquire_next_image().unwrap()
        };

        match result {
            Ok((image_index, _is_subopt)) => {
                // if is_subopt {
                //     log::warn!("Vulkan swapchain is suboptimal for surface");
                //     self.recreate_swapchain();
                // }

                self.record_command_buffers(image_index as usize);

                self.swapchain.submit_command_buffers(
                    self.command_buffers[image_index as usize],
                    image_index as usize,
                ).unwrap();
            },
            Err(ash::vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                log::error!("Out of date KHR");
                self.recreate_swapchain();
                return;
            }
            Err(_) => {
                log::error!("Failed to acquire next image");
                panic!("Failed to acquire next image");
            },
        }
    }

    pub fn resize(&mut self) {
        self.recreate_swapchain();
    }
}

impl Drop for App {
    fn drop(&mut self) {
        unsafe {
            self.device.logical_device.destroy_pipeline_layout(self.pipeline_layout, None);
        }
    }
}
