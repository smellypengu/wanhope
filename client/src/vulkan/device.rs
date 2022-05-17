use std::{
    ffi::{CStr, CString},
    rc::Rc,
};

use super::{Instance, RenderError, ENABLE_VALIDATION_LAYERS};

pub struct SwapchainSupportDetails {
    pub capabilities: ash::vk::SurfaceCapabilitiesKHR,
    pub formats: Vec<ash::vk::SurfaceFormatKHR>,
    pub present_modes: Vec<ash::vk::PresentModeKHR>,
}

pub struct QueueFamilyIndices {
    pub graphics_family: u32,
    pub present_family: u32,
    graphics_family_has_value: bool,
    present_family_has_value: bool,
}

impl QueueFamilyIndices {
    pub fn is_complete(&self) -> bool {
        self.graphics_family_has_value && self.present_family_has_value
    }
}

pub struct Device {
    pub instance: Instance,
    surface: ash::extensions::khr::Surface,
    pub surface_khr: ash::vk::SurfaceKHR,
    physical_device: ash::vk::PhysicalDevice,
    properties: ash::vk::PhysicalDeviceProperties,
    pub logical_device: ash::Device,
    pub command_pool: ash::vk::CommandPool,
    pub graphics_queue: ash::vk::Queue,
    pub present_queue: ash::vk::Queue,
}

impl Device {
    pub fn new(
        app_name: CString,
        engine_name: CString,
        window: &winit::window::Window,
    ) -> anyhow::Result<Rc<Self>, RenderError> {
        let instance = Instance::new(app_name, engine_name)?;
        log::debug!("Vulkan instance created");

        let (surface, surface_khr) = Self::create_surface(&instance, window)?;
        log::debug!("Vulkan surface created");

        let (physical_device, properties) =
            Self::pick_physical_device(&instance, &surface, surface_khr)?;
        log::debug!("Vulkan physical device created");

        let (logical_device, graphics_queue, present_queue) =
            Self::create_logical_device(&instance, &surface, surface_khr, physical_device)?;
        log::debug!("Vulkan logical device created");

        let command_pool = Self::create_command_pool(
            &instance,
            &surface,
            surface_khr,
            physical_device,
            &logical_device,
        )?;

        Ok(Rc::new(Self {
            instance,
            surface,
            surface_khr,
            physical_device,
            properties,
            logical_device,
            command_pool,
            graphics_queue,
            present_queue,
        }))
    }

    pub fn get_swapchain_support(&self) -> anyhow::Result<SwapchainSupportDetails, RenderError> {
        Ok(Self::query_swapchain_support(
            &self.surface,
            self.surface_khr,
            self.physical_device,
        )?)
    }

    pub fn find_memory_type(
        &self,
        type_filter: u32,
        properties: ash::vk::MemoryPropertyFlags,
    ) -> Option<(u32, bool)> {
        let mem_properties = unsafe {
            self.instance
                .inner()
                .get_physical_device_memory_properties(self.physical_device)
        };

        let mut memory_type = None;

        for (i, m_type) in mem_properties.memory_types.iter().enumerate() {
            if (type_filter) & (1 << i) != 0 && (m_type.property_flags & properties) == properties {
                memory_type = Some((
                    i as u32,
                    m_type
                        .property_flags
                        .contains(ash::vk::MemoryPropertyFlags::HOST_COHERENT),
                ));

                break;
            }
        }

        memory_type
    }

    pub fn find_physical_queue_families(&self) -> anyhow::Result<QueueFamilyIndices, RenderError> {
        Ok(Self::find_queue_families(
            &self.instance,
            &self.surface,
            self.surface_khr,
            self.physical_device,
        )?)
    }

    pub fn find_supported_format(
        &self,
        candidates: &Vec<ash::vk::Format>,
        tiling: ash::vk::ImageTiling,
        features: ash::vk::FormatFeatureFlags,
    ) -> ash::vk::Format {
        *candidates
            .iter()
            .find(|format| {
                let properties = unsafe {
                    self.instance
                        .inner()
                        .get_physical_device_format_properties(self.physical_device, **format)
                };

                if tiling == ash::vk::ImageTiling::LINEAR {
                    return (properties.linear_tiling_features & features) == features;
                } else if tiling == ash::vk::ImageTiling::OPTIMAL {
                    return (properties.optimal_tiling_features & features) == features;
                }

                false
            })
            .expect("Failed to find supported format!")
    }

    pub fn create_buffer(
        &self,
        size: ash::vk::DeviceSize,
        usage: ash::vk::BufferUsageFlags,
        properties: ash::vk::MemoryPropertyFlags,
    ) -> anyhow::Result<(ash::vk::Buffer, ash::vk::DeviceMemory, bool), RenderError> {
        let create_info = ash::vk::BufferCreateInfo::builder()
            .size(size)
            .usage(usage)
            .sharing_mode(ash::vk::SharingMode::EXCLUSIVE);

        let buffer = unsafe { self.logical_device.create_buffer(&create_info, None)? };

        let mem_requirements =
            unsafe { self.logical_device.get_buffer_memory_requirements(buffer) };

        let (memory_type, coherent) = self
            .find_memory_type(mem_requirements.memory_type_bits, properties)
            .unwrap();

        let alloc_info = ash::vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(memory_type);

        let buffer_memory = unsafe { self.logical_device.allocate_memory(&alloc_info, None)? };

        unsafe {
            self.logical_device
                .bind_buffer_memory(buffer, buffer_memory, 0)?
        };

        Ok((buffer, buffer_memory, coherent))
    }

    pub fn begin_single_time_commands(
        &self,
    ) -> anyhow::Result<ash::vk::CommandBuffer, RenderError> {
        let alloc_info = ash::vk::CommandBufferAllocateInfo::builder()
            .level(ash::vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.command_pool)
            .command_buffer_count(1);

        let command_buffer =
            unsafe { self.logical_device.allocate_command_buffers(&alloc_info)?[0] };

        let begin_info = ash::vk::CommandBufferBeginInfo::builder()
            .flags(ash::vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);

        unsafe {
            self.logical_device
                .begin_command_buffer(command_buffer, &begin_info)?
        };

        Ok(command_buffer)
    }

    pub fn end_single_time_commands(
        &self,
        command_buffer: ash::vk::CommandBuffer,
    ) -> anyhow::Result<(), RenderError> {
        unsafe {
            self.logical_device.end_command_buffer(command_buffer)?;

            let submit_info = ash::vk::SubmitInfo::builder()
                .command_buffers(std::slice::from_ref(&command_buffer));

            self.logical_device.queue_submit(
                self.graphics_queue,
                std::slice::from_ref(&submit_info),
                ash::vk::Fence::null(),
            )?;

            self.logical_device.queue_wait_idle(self.graphics_queue)?;

            self.logical_device
                .free_command_buffers(self.command_pool, &[command_buffer]);
        }

        Ok(())
    }

    pub fn copy_buffer(
        &self,
        src_buffer: ash::vk::Buffer,
        dst_buffer: ash::vk::Buffer,
        size: ash::vk::DeviceSize,
    ) -> anyhow::Result<(), RenderError> {
        let command_buffer = self.begin_single_time_commands()?;

        let copy_region = ash::vk::BufferCopy::builder()
            .src_offset(0)
            .dst_offset(0)
            .size(size);

        unsafe {
            self.logical_device.cmd_copy_buffer(
                command_buffer,
                src_buffer,
                dst_buffer,
                std::slice::from_ref(&copy_region),
            )
        };

        self.end_single_time_commands(command_buffer)?;

        Ok(())
    }

    pub fn create_image_with_info(
        &self,
        image_info: &ash::vk::ImageCreateInfo,
        properties: ash::vk::MemoryPropertyFlags,
    ) -> anyhow::Result<(ash::vk::Image, ash::vk::DeviceMemory), RenderError> {
        let image = unsafe { self.logical_device.create_image(image_info, None)? };

        let mem_requirements = unsafe { self.logical_device.get_image_memory_requirements(image) };

        let alloc_info = ash::vk::MemoryAllocateInfo::builder()
            .allocation_size(mem_requirements.size)
            .memory_type_index(
                self.find_memory_type(mem_requirements.memory_type_bits, properties)
                    .unwrap()
                    .0,
            );

        let image_memory = unsafe { self.logical_device.allocate_memory(&alloc_info, None)? };

        unsafe {
            self.logical_device
                .bind_image_memory(image, image_memory, 0)?
        }

        Ok((image, image_memory))
    }

    fn create_surface(
        instance: &Instance,
        window: &winit::window::Window,
    ) -> anyhow::Result<(ash::extensions::khr::Surface, ash::vk::SurfaceKHR), RenderError> {
        let surface = ash::extensions::khr::Surface::new(&instance.entry, &instance.inner());

        let surface_khr =
            unsafe { ash_window::create_surface(&instance.entry, instance.inner(), window, None)? };

        Ok((surface, surface_khr))
    }

    fn pick_physical_device(
        instance: &Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
    ) -> anyhow::Result<(ash::vk::PhysicalDevice, ash::vk::PhysicalDeviceProperties), RenderError>
    {
        let physical_devices = unsafe { instance.inner().enumerate_physical_devices()? };

        log::debug!("Physical device count: {}", physical_devices.len());

        let physical_device = physical_devices
            .into_iter()
            .find(|physical_device| {
                Self::is_physical_device_suitable(instance, surface, surface_khr, *physical_device)
                    .unwrap()
            }) // TODO: fix unwrap?
            .expect("No suitable physical device found");

        let physical_device_properties = unsafe {
            instance
                .inner()
                .get_physical_device_properties(physical_device)
        };

        log::debug!("Selected physical device: {:?}", unsafe {
            CStr::from_ptr(physical_device_properties.device_name.as_ptr())
        });

        Ok((physical_device, physical_device_properties))
    }

    fn is_physical_device_suitable(
        instance: &Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<bool, RenderError> {
        let queue_indices =
            Self::find_queue_families(instance, surface, surface_khr, physical_device)?;

        let extensions_supported =
            Self::check_physical_device_extension_support(instance, physical_device)?;

        let mut swapchain_adequate = false;

        if extensions_supported {
            let swapchain_support =
                Self::query_swapchain_support(surface, surface_khr, physical_device)?;

            swapchain_adequate = {
                !swapchain_support.formats.is_empty() && !swapchain_support.present_modes.is_empty()
            }
        }

        let supported_features = unsafe {
            instance
                .inner()
                .get_physical_device_features(physical_device)
        };

        Ok({
            queue_indices.is_complete()
                && extensions_supported
                && swapchain_adequate
                && supported_features.sampler_anisotropy != 0
        })
    }

    fn create_logical_device(
        instance: &Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<(ash::Device, ash::vk::Queue, ash::vk::Queue), RenderError> {
        let queue_indices =
            Self::find_queue_families(instance, surface, surface_khr, physical_device)?;

        let queue_priorities = [1.0f32];

        let queue_create_infos = {
            let mut indices = vec![queue_indices.graphics_family, queue_indices.present_family];
            indices.dedup();

            indices
                .iter()
                .map(|index| {
                    ash::vk::DeviceQueueCreateInfo::builder()
                        .queue_family_index(*index)
                        .queue_priorities(&queue_priorities)
                        .build()
                })
                .collect::<Vec<_>>()
        };

        let physical_device_features = ash::vk::PhysicalDeviceFeatures::builder();

        let (_, logical_device_extensions_ptrs) = Self::get_device_extensions();

        let mut create_info = ash::vk::DeviceCreateInfo::builder()
            .queue_create_infos(&queue_create_infos)
            .enabled_features(&physical_device_features)
            .enabled_extension_names(&logical_device_extensions_ptrs);

        let (_layer_names, layer_name_ptrs) = Instance::get_enabled_layers();

        if ENABLE_VALIDATION_LAYERS {
            create_info = create_info.enabled_layer_names(&layer_name_ptrs);
        }

        let logical_device = unsafe {
            instance
                .inner()
                .create_device(physical_device, &create_info, None)?
        };

        let graphics_queue =
            unsafe { logical_device.get_device_queue(queue_indices.graphics_family, 0) };

        let present_queue =
            unsafe { logical_device.get_device_queue(queue_indices.present_family, 0) };

        Ok((logical_device, graphics_queue, present_queue))
    }

    fn create_command_pool(
        instance: &Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
        physical_device: ash::vk::PhysicalDevice,
        logical_device: &ash::Device,
    ) -> anyhow::Result<ash::vk::CommandPool, RenderError> {
        let queue_indices =
            Self::find_queue_families(instance, surface, surface_khr, physical_device)?;

        let create_info = ash::vk::CommandPoolCreateInfo::builder()
            .queue_family_index(queue_indices.graphics_family)
            .flags(
                ash::vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER
                    | ash::vk::CommandPoolCreateFlags::TRANSIENT,
            );

        Ok(unsafe { logical_device.create_command_pool(&create_info, None)? })
    }

    fn get_device_extensions() -> ([&'static CStr; 1], Vec<*const i8>) {
        let device_extensions: [&'static CStr; 1] = [ash::extensions::khr::Swapchain::name()];

        let ext_names_ptrs = device_extensions
            .iter()
            .map(|ext| ext.as_ptr())
            .collect::<Vec<_>>();

        (device_extensions, ext_names_ptrs)
    }

    fn check_physical_device_extension_support(
        instance: &Instance,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<bool, RenderError> {
        let available_extensions = unsafe {
            instance
                .inner()
                .enumerate_device_extension_properties(physical_device)?
        };

        let (required_extensions, _) = Self::get_device_extensions();

        for extension in required_extensions.iter() {
            let found = available_extensions.iter().any(|ext| {
                let name = unsafe { CStr::from_ptr(ext.extension_name.as_ptr()) };

                extension == &name
            });

            if !found {
                log::error!(
                    "Physical Device does not support the following extension: {:?}",
                    extension
                );

                return Ok(false);
            }
        }

        Ok(true)
    }

    fn find_queue_families(
        instance: &Instance,
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<QueueFamilyIndices, RenderError> {
        let mut graphics_family = 0;
        let mut present_family = 0;
        let mut graphics_family_has_value = false;
        let mut present_family_has_value = false;

        let queue_families = unsafe {
            instance
                .inner()
                .get_physical_device_queue_family_properties(physical_device)
        };

        for (index, queue_family) in queue_families
            .iter()
            .filter(|f| f.queue_count > 0)
            .enumerate()
        {
            let index = index as u32;

            if queue_family
                .queue_flags
                .contains(ash::vk::QueueFlags::GRAPHICS)
            {
                graphics_family = index;
                graphics_family_has_value = true;
            }

            let present_support = unsafe {
                surface.get_physical_device_surface_support(physical_device, index, surface_khr)?
            };

            if present_support {
                present_family = index;
                present_family_has_value = true;
            }

            if graphics_family_has_value && present_family_has_value {
                break;
            }
        }

        Ok(QueueFamilyIndices {
            graphics_family,
            present_family,
            graphics_family_has_value,
            present_family_has_value,
        })
    }

    fn query_swapchain_support(
        surface: &ash::extensions::khr::Surface,
        surface_khr: ash::vk::SurfaceKHR,
        physical_device: ash::vk::PhysicalDevice,
    ) -> anyhow::Result<SwapchainSupportDetails, RenderError> {
        let capabilities = unsafe {
            surface.get_physical_device_surface_capabilities(physical_device, surface_khr)?
        };

        let formats =
            unsafe { surface.get_physical_device_surface_formats(physical_device, surface_khr)? };

        let present_modes = unsafe {
            surface.get_physical_device_surface_present_modes(physical_device, surface_khr)?
        };

        Ok(SwapchainSupportDetails {
            capabilities,
            formats,
            present_modes,
        })
    }
}

impl Drop for Device {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan device");

        unsafe {
            self.logical_device
                .destroy_command_pool(self.command_pool, None);

            self.logical_device.destroy_device(None);

            self.surface.destroy_surface(self.surface_khr, None);
        }
    }
}
