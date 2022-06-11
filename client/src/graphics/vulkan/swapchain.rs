use std::rc::Rc;

use crate::graphics::RenderError;

use super::{Device, ImageView};

pub const MAX_FRAMES_IN_FLIGHT: usize = 2;

pub struct Swapchain {
    device: Rc<Device>,
    swapchain: ash::extensions::khr::Swapchain,
    pub swapchain_khr: ash::vk::SwapchainKHR,
    pub swapchain_image_format: ash::vk::Format,
    swapchain_depth_format: ash::vk::Format,
    pub swapchain_extent: ash::vk::Extent2D,
    pub swapchain_images: Vec<ash::vk::Image>,
    swapchain_image_views: Vec<Rc<ImageView>>,
    pub swapchain_framebuffers: Vec<ash::vk::Framebuffer>,
    pub render_pass: ash::vk::RenderPass,
    depth_images: Vec<ash::vk::Image>,
    depth_image_memories: Vec<ash::vk::DeviceMemory>,
    depth_image_views: Vec<Rc<ImageView>>,
    image_available_semaphores: Vec<ash::vk::Semaphore>,
    render_finished_semaphores: Vec<ash::vk::Semaphore>,
    in_flight_fences: Vec<ash::vk::Fence>,
    images_in_flight: Vec<ash::vk::Fence>,
    current_frame: usize,
}

impl Swapchain {
    pub unsafe fn new(
        device: Rc<Device>,
        window_extent: ash::vk::Extent2D,
        old_swapchain: Option<ash::vk::SwapchainKHR>,
    ) -> anyhow::Result<Self, RenderError> {
        let old_swapchain = match old_swapchain {
            Some(swapchain) => swapchain,
            None => ash::vk::SwapchainKHR::null(),
        };

        let (swapchain, swapchain_khr, swapchain_images, swapchain_image_format, swapchain_extent) =
            Self::create_swapchain(&device, window_extent, old_swapchain)?;
        log::debug!("Vulkan swapchain created");

        let swapchain_image_views =
            Self::create_image_views(device.clone(), &swapchain_images, swapchain_image_format);

        let render_pass = Self::create_render_pass(&device, swapchain_image_format)?;
        log::debug!("Vulkan render pass created");

        let (depth_images, depth_image_memories, depth_image_views, swapchain_depth_format) =
            Self::create_depth_resources(device.clone(), &swapchain_images, swapchain_extent);
        log::debug!("Vulkan depth resources created");

        let swapchain_framebuffers = Self::create_framebuffers(
            &device.logical_device,
            swapchain_extent,
            &swapchain_image_views,
            &depth_image_views,
            render_pass,
        );
        log::debug!("Vulkan framebuffers created");

        let (
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
        ) = Self::create_sync_objects(&device.logical_device, &swapchain_images)?;
        log::debug!("Vulkan sync objects created");

        Ok(Self {
            device,
            swapchain,
            swapchain_khr,
            swapchain_image_format,
            swapchain_depth_format,
            swapchain_extent,
            swapchain_images,
            swapchain_image_views,
            swapchain_framebuffers,
            render_pass,
            depth_images,
            depth_image_memories,
            depth_image_views,
            image_available_semaphores,
            render_finished_semaphores,
            in_flight_fences,
            images_in_flight,
            current_frame: 0,
        })
    }

    pub fn compare_swap_formats(&self, other_swapchain: &Self) -> anyhow::Result<(), RenderError> {
        if other_swapchain.swapchain_depth_format == self.swapchain_depth_format
            && other_swapchain.swapchain_image_format == self.swapchain_image_format
        {
            Ok(())
        } else {
            Err(RenderError::CompareSwapFormatsError)
        }
    }

    pub unsafe fn find_depth_format(device: &Rc<Device>) -> ash::vk::Format {
        let candidates = vec![
            ash::vk::Format::D32_SFLOAT,
            ash::vk::Format::D32_SFLOAT_S8_UINT,
            ash::vk::Format::D32_SFLOAT_S8_UINT,
        ];

        device.find_supported_format(
            &candidates,
            ash::vk::ImageTiling::OPTIMAL,
            ash::vk::FormatFeatureFlags::DEPTH_STENCIL_ATTACHMENT,
        )
    }

    pub unsafe fn acquire_next_image(
        &self,
    ) -> anyhow::Result<Result<(u32, bool), ash::vk::Result>, RenderError> {
        self.device.logical_device.wait_for_fences(
            &[self.in_flight_fences[self.current_frame]],
            false,
            u64::MAX,
        )?;

        Ok(self.swapchain.acquire_next_image(
            self.swapchain_khr,
            u64::MAX,
            self.image_available_semaphores[self.current_frame],
            ash::vk::Fence::null(),
        ))
    }

    pub unsafe fn submit_command_buffers(
        &mut self,
        buffer: ash::vk::CommandBuffer,
        image_index: usize,
    ) -> anyhow::Result<bool, RenderError> {
        if self.images_in_flight[image_index] != ash::vk::Fence::null() {
            self.device.logical_device.wait_for_fences(
                &[self.images_in_flight[image_index]],
                true,
                u64::MAX,
            )?
        }

        self.images_in_flight[image_index] = self.in_flight_fences[self.current_frame];

        let wait_semaphores = [self.image_available_semaphores[self.current_frame]];

        let wait_stages = [ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT];

        let signal_semaphores = [self.render_finished_semaphores[self.current_frame]];

        let submit_info = &[ash::vk::SubmitInfo::builder()
            .wait_semaphores(&wait_semaphores)
            .wait_dst_stage_mask(&wait_stages)
            .command_buffers(&[buffer])
            .signal_semaphores(&signal_semaphores)
            .build()];

        self.device
            .logical_device
            .reset_fences(&[self.in_flight_fences[self.current_frame]])?;

        self.device.logical_device.queue_submit(
            self.device.graphics_queue,
            submit_info,
            self.in_flight_fences[self.current_frame],
        )?;

        let swapchains = [self.swapchain_khr];

        let image_index = image_index as u32;

        let present_info = ash::vk::PresentInfoKHR::builder()
            .wait_semaphores(&signal_semaphores)
            .swapchains(&swapchains)
            .image_indices(std::slice::from_ref(&image_index));

        self.current_frame = (self.current_frame + 1) % MAX_FRAMES_IN_FLIGHT;

        Ok(self
            .swapchain
            .queue_present(self.device.present_queue, &present_info)?)
    }

    unsafe fn create_swapchain(
        device: &Rc<Device>,
        window_extent: ash::vk::Extent2D,
        old_swapchain: ash::vk::SwapchainKHR,
    ) -> anyhow::Result<
        (
            ash::extensions::khr::Swapchain,
            ash::vk::SwapchainKHR,
            Vec<ash::vk::Image>,
            ash::vk::Format,
            ash::vk::Extent2D,
        ),
        RenderError,
    > {
        let swapchain_support = device.get_swapchain_support()?;

        let surface_format = Self::choose_surface_format(&swapchain_support.formats);
        log::debug!("Vulkan surface format: {:?}", surface_format);

        let present_mode = Self::choose_present_mode(&swapchain_support.present_modes);
        log::debug!("Vulkan present mode: {:?}", present_mode);

        let extent = Self::choose_extent(&swapchain_support.capabilities, window_extent);

        let mut image_count = swapchain_support.capabilities.min_image_count + 1;

        if swapchain_support.capabilities.max_image_count > 0
            && image_count > swapchain_support.capabilities.max_image_count
        {
            image_count = swapchain_support.capabilities.max_image_count;
        }

        let mut create_info = ash::vk::SwapchainCreateInfoKHR::builder()
            .surface(device.surface_khr)
            .min_image_count(image_count)
            .image_format(surface_format.format)
            .image_color_space(surface_format.color_space)
            .image_extent(extent)
            .image_array_layers(1)
            .image_usage(ash::vk::ImageUsageFlags::COLOR_ATTACHMENT);

        let queue_indices = device.find_physical_queue_families()?;

        let queue_family_indices = [queue_indices.graphics_family, queue_indices.present_family];

        if queue_indices.graphics_family != queue_indices.present_family {
            create_info = create_info
                .image_sharing_mode(ash::vk::SharingMode::CONCURRENT)
                .queue_family_indices(&queue_family_indices);
        } else {
            create_info = create_info.image_sharing_mode(ash::vk::SharingMode::EXCLUSIVE);
        }

        let create_info = create_info
            .pre_transform(swapchain_support.capabilities.current_transform)
            .composite_alpha(ash::vk::CompositeAlphaFlagsKHR::OPAQUE)
            .present_mode(present_mode)
            .clipped(true)
            .old_swapchain(old_swapchain);

        let swapchain =
            ash::extensions::khr::Swapchain::new(&device.instance.inner(), &device.logical_device);

        let swapchain_khr = swapchain.create_swapchain(&create_info, None)?;

        let swapchain_images = swapchain.get_swapchain_images(swapchain_khr)?;

        let swapchain_image_format = surface_format.format;

        let swapchain_extent = extent;

        Ok((
            swapchain,
            swapchain_khr,
            swapchain_images,
            swapchain_image_format,
            swapchain_extent,
        ))
    }

    fn create_image_views(
        device: Rc<Device>,
        swapchain_images: &Vec<ash::vk::Image>,
        swapchain_image_format: ash::vk::Format,
    ) -> Vec<Rc<ImageView>> {
        swapchain_images
            .iter()
            .map(|image| {
                ImageView::new(
                    device.clone(),
                    *image,
                    swapchain_image_format,
                    ash::vk::ImageAspectFlags::COLOR,
                )
                .unwrap() // fix unwrap?
            })
            .collect::<Vec<_>>()
    }

    unsafe fn create_depth_resources(
        device: Rc<Device>,
        swapchain_images: &Vec<ash::vk::Image>,
        swapchain_extent: ash::vk::Extent2D,
    ) -> (
        Vec<ash::vk::Image>,
        Vec<ash::vk::DeviceMemory>,
        Vec<Rc<ImageView>>,
        ash::vk::Format,
    ) {
        let depth_format = Self::find_depth_format(&device);

        let (images, image_memories): (Vec<ash::vk::Image>, Vec<ash::vk::DeviceMemory>) =
            swapchain_images
                .iter()
                .map(|_| {
                    let extent = ash::vk::Extent3D {
                        width: swapchain_extent.width,
                        height: swapchain_extent.height,
                        depth: 1,
                    };

                    let image_info = ash::vk::ImageCreateInfo::builder()
                        .image_type(ash::vk::ImageType::TYPE_2D)
                        .extent(extent)
                        .mip_levels(1)
                        .array_layers(1)
                        .format(depth_format)
                        .tiling(ash::vk::ImageTiling::OPTIMAL)
                        .initial_layout(ash::vk::ImageLayout::UNDEFINED)
                        .usage(ash::vk::ImageUsageFlags::DEPTH_STENCIL_ATTACHMENT)
                        .samples(ash::vk::SampleCountFlags::TYPE_1)
                        .sharing_mode(ash::vk::SharingMode::EXCLUSIVE);

                    device
                        .create_image_with_info(
                            &image_info,
                            ash::vk::MemoryPropertyFlags::DEVICE_LOCAL,
                        )
                        .unwrap() // TODO: fix unwrap?
                })
                .unzip();

        let image_views = images
            .iter()
            .map(|image| {
                ImageView::new(
                    device.clone(),
                    *image,
                    depth_format,
                    ash::vk::ImageAspectFlags::DEPTH,
                )
                .unwrap() // fix unwrap?
            })
            .collect::<Vec<_>>();

        (images, image_memories, image_views, depth_format)
    }

    unsafe fn create_render_pass(
        device: &Rc<Device>,
        swapchain_image_format: ash::vk::Format,
    ) -> anyhow::Result<ash::vk::RenderPass, RenderError> {
        Ok(device.logical_device.create_render_pass(
            &ash::vk::RenderPassCreateInfo::builder()
                .attachments(&[
                    ash::vk::AttachmentDescription {
                        format: swapchain_image_format,
                        samples: ash::vk::SampleCountFlags::TYPE_1,
                        load_op: ash::vk::AttachmentLoadOp::CLEAR,
                        store_op: ash::vk::AttachmentStoreOp::STORE,
                        stencil_load_op: ash::vk::AttachmentLoadOp::DONT_CARE,
                        stencil_store_op: ash::vk::AttachmentStoreOp::DONT_CARE,
                        initial_layout: ash::vk::ImageLayout::UNDEFINED,
                        final_layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                        ..Default::default()
                    },
                    ash::vk::AttachmentDescription {
                        format: Self::find_depth_format(device),
                        samples: ash::vk::SampleCountFlags::TYPE_1,
                        load_op: ash::vk::AttachmentLoadOp::CLEAR,
                        store_op: ash::vk::AttachmentStoreOp::DONT_CARE,
                        stencil_load_op: ash::vk::AttachmentLoadOp::DONT_CARE,
                        stencil_store_op: ash::vk::AttachmentStoreOp::DONT_CARE,
                        initial_layout: ash::vk::ImageLayout::UNDEFINED,
                        final_layout: ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                        ..Default::default()
                    },
                ])
                .subpasses(&[ash::vk::SubpassDescription::builder()
                    .pipeline_bind_point(ash::vk::PipelineBindPoint::GRAPHICS)
                    .color_attachments(&[ash::vk::AttachmentReference {
                        attachment: 0,
                        layout: ash::vk::ImageLayout::COLOR_ATTACHMENT_OPTIMAL,
                    }])
                    .depth_stencil_attachment(&ash::vk::AttachmentReference {
                        attachment: 1,
                        layout: ash::vk::ImageLayout::DEPTH_STENCIL_ATTACHMENT_OPTIMAL,
                    })
                    .build()])
                .dependencies(&[ash::vk::SubpassDependency {
                    src_subpass: ash::vk::SUBPASS_EXTERNAL,
                    dst_subpass: 0,
                    src_stage_mask: ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                        | ash::vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
                    dst_stage_mask: ash::vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT
                        | ash::vk::PipelineStageFlags::EARLY_FRAGMENT_TESTS,
                    dst_access_mask: ash::vk::AccessFlags::COLOR_ATTACHMENT_WRITE
                        | ash::vk::AccessFlags::DEPTH_STENCIL_ATTACHMENT_WRITE,
                    ..Default::default()
                }]),
            None,
        )?)
    }

    unsafe fn create_framebuffers(
        logical_device: &ash::Device,
        swapchain_extent: ash::vk::Extent2D,
        swapchain_image_views: &Vec<Rc<ImageView>>,
        depth_image_views: &Vec<Rc<ImageView>>,
        render_pass: ash::vk::RenderPass,
    ) -> Vec<ash::vk::Framebuffer> {
        swapchain_image_views
            .iter()
            .zip(depth_image_views)
            .map(|view| [view.0.inner(), view.1.inner()])
            .map(|attachments| {
                let framebuffer_info = ash::vk::FramebufferCreateInfo::builder()
                    .render_pass(render_pass)
                    .attachments(&attachments)
                    .width(swapchain_extent.width)
                    .height(swapchain_extent.height)
                    .layers(1);

                logical_device
                    .create_framebuffer(&framebuffer_info, None)
                    .map_err(|e| log::error!("Unable to create framebuffer: {}", e))
                    .unwrap() // TODO: fix unwrap?
            })
            .collect::<Vec<_>>()
    }

    unsafe fn create_sync_objects(
        logical_device: &ash::Device,
        swapchain_images: &Vec<ash::vk::Image>,
    ) -> anyhow::Result<
        (
            Vec<ash::vk::Semaphore>,
            Vec<ash::vk::Semaphore>,
            Vec<ash::vk::Fence>,
            Vec<ash::vk::Fence>,
        ),
        RenderError,
    > {
        let semaphore_info = ash::vk::SemaphoreCreateInfo::builder();

        let fence_info =
            ash::vk::FenceCreateInfo::builder().flags(ash::vk::FenceCreateFlags::SIGNALED);

        let mut image_available_semaphores = Vec::new();
        let mut render_finished_semaphore = Vec::new();
        let mut in_flight_fences = Vec::new();

        for _ in 0..MAX_FRAMES_IN_FLIGHT {
            image_available_semaphores
                .push(logical_device.create_semaphore(&semaphore_info, None)?);

            render_finished_semaphore.push(logical_device.create_semaphore(&semaphore_info, None)?);

            in_flight_fences.push(logical_device.create_fence(&fence_info, None)?);
        }

        let images_in_flight = vec![ash::vk::Fence::null(); swapchain_images.len()];

        Ok((
            image_available_semaphores,
            render_finished_semaphore,
            in_flight_fences,
            images_in_flight,
        ))
    }

    fn choose_surface_format(
        available_formats: &Vec<ash::vk::SurfaceFormatKHR>,
    ) -> ash::vk::SurfaceFormatKHR {
        let format = available_formats
            .iter()
            .map(|f| *f)
            .find(|available_format| {
                available_format.format == ash::vk::Format::B8G8R8A8_SRGB
                    && available_format.color_space == ash::vk::ColorSpaceKHR::SRGB_NONLINEAR
            })
            .unwrap_or_else(|| {
                log::warn!(
                    "Could not find appropriate surface format, returning first available format"
                );
                available_formats[0]
            });

        format
    }

    fn choose_present_mode(
        available_present_modes: &Vec<ash::vk::PresentModeKHR>,
    ) -> ash::vk::PresentModeKHR {
        let present_mode = available_present_modes
            .iter()
            .map(|pm| *pm)
            // .find(|available_present_mode| *available_present_mode == ash::vk::PresentModeKHR::MAILBOX)
            // .find(|available_present_mode| *available_present_mode == ash::vk::PresentModeKHR::IMMEDIATE)
            .find(|available_present_mode| *available_present_mode == ash::vk::PresentModeKHR::FIFO)
            .unwrap_or_else(|| {
                log::warn!("Could not find desired present mode, defaulting to FIFO");
                ash::vk::PresentModeKHR::FIFO
            });

        present_mode
    }

    fn choose_extent(
        capabilities: &ash::vk::SurfaceCapabilitiesKHR,
        window_extent: ash::vk::Extent2D,
    ) -> ash::vk::Extent2D {
        if capabilities.current_extent.width != u32::MAX {
            capabilities.current_extent
        } else {
            ash::vk::Extent2D {
                width: std::cmp::max(
                    capabilities.min_image_extent.width,
                    std::cmp::min(capabilities.max_image_extent.width, window_extent.width),
                ),
                height: std::cmp::max(
                    capabilities.min_image_extent.height,
                    std::cmp::min(capabilities.max_image_extent.height, window_extent.height),
                ),
            }
        }
    }

    #[inline]
    pub fn width(&self) -> u32 {
        self.swapchain_extent.width
    }

    #[inline]
    pub fn height(&self) -> u32 {
        self.swapchain_extent.height
    }

    #[inline]
    pub fn extent_aspect_ratio(&self) -> f32 {
        self.swapchain_extent.width as f32 / self.swapchain_extent.height as f32
    }
}

impl Drop for Swapchain {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan swapchain");

        unsafe {
            // TODO: figure out why this line causes the rest of the app not to be destroyed properly
            // self.swapchain.destroy_swapchain(self.swapchain_khr, None);

            self.depth_images
                .iter()
                .for_each(|i| self.device.logical_device.destroy_image(*i, None));

            self.depth_image_memories
                .iter()
                .for_each(|m| self.device.logical_device.free_memory(*m, None));

            self.swapchain_framebuffers
                .iter()
                .for_each(|f| self.device.logical_device.destroy_framebuffer(*f, None));

            self.device
                .logical_device
                .destroy_render_pass(self.render_pass, None);

            self.render_finished_semaphores
                .iter()
                .for_each(|s| self.device.logical_device.destroy_semaphore(*s, None));

            self.image_available_semaphores
                .iter()
                .for_each(|s| self.device.logical_device.destroy_semaphore(*s, None));

            self.in_flight_fences
                .iter()
                .for_each(|f| self.device.logical_device.destroy_fence(*f, None));
        }
    }
}
