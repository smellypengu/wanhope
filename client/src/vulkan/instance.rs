use std::ffi::{CString, CStr, c_void};

use super::RenderError;

pub const ENABLE_VALIDATION_LAYERS: bool = true;

const VALIDATION_LAYERS: [&'static str; 1] = ["VK_LAYER_KHRONOS_validation"];

unsafe extern "system" fn vulkan_debug_callback(
    flag: ash::vk::DebugUtilsMessageSeverityFlagsEXT,
    typ: ash::vk::DebugUtilsMessageTypeFlagsEXT,
    p_callback_data: *const ash::vk::DebugUtilsMessengerCallbackDataEXT,
    _p_user_data: *mut c_void,
) -> ash::vk::Bool32 {
    let message = CStr::from_ptr((*p_callback_data).p_message);

    if flag == ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR {
        log::error!("{:?} - {:?}", typ, message);
    } else if flag == ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING {
        log::warn!("{:?} - {:?}", typ, message);
    } else if flag == ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO {
        log::info!("{:?} - {:?}", typ, message);
    }  else {
        log::info!("{:?} - {:?}", typ, message);
    }

    ash::vk::FALSE
}

pub struct Instance {
    pub entry: ash::Entry,
    instance: ash::Instance,
    debug_messenger: Option<(ash::extensions::ext::DebugUtils, ash::vk::DebugUtilsMessengerEXT)>,
}

impl Instance {
    pub fn new(
        app_name: CString,
        engine_name: CString,
    ) -> anyhow::Result<Self, RenderError> {
        let entry = unsafe {
            ash::Entry::load()?
        };

        let app_info = ash::vk::ApplicationInfo::builder()
            .application_name(app_name.as_c_str())
            .application_version(ash::vk::make_api_version(0, 0, 1, 0))
            .engine_name(engine_name.as_c_str())
            .engine_version(ash::vk::make_api_version(0, 0, 1, 0))
            .api_version(ash::vk::make_api_version(0, 1, 3, 212));

        let extensions = Self::get_required_extensions();

        let mut create_info = ash::vk::InstanceCreateInfo::builder()
            .application_info(&app_info)
            .enabled_extension_names(&extensions);

        let (_layer_names, layer_name_ptrs) = Self::get_enabled_layers();

        if ENABLE_VALIDATION_LAYERS {
            if Self::check_validation_layer_support(&entry)? {
                create_info = create_info.enabled_layer_names(&layer_name_ptrs);
            } else {
                panic!("Validation layers requested, but not available!");
            }
        }

        let instance = unsafe {
            entry.create_instance(&create_info, None)?
        };

        let debug_messenger = if ENABLE_VALIDATION_LAYERS {
            Some(Self::setup_debug_messenger(&entry, &instance)?)
        } else {
            None
        };

        Ok(Self {
            entry,
            instance,
            debug_messenger,
        })
    }

    fn setup_debug_messenger(
        entry: &ash::Entry,
        instance: &ash::Instance,
    ) -> anyhow::Result<(ash::extensions::ext::DebugUtils, ash::vk::DebugUtilsMessengerEXT), RenderError> {
        let create_info = ash::vk::DebugUtilsMessengerCreateInfoEXT::builder()
            .message_severity(
                ash::vk::DebugUtilsMessageSeverityFlagsEXT::ERROR
                    | ash::vk::DebugUtilsMessageSeverityFlagsEXT::WARNING
                    // | ash::vk::DebugUtilsMessageSeverityFlagsEXT::INFO
            ).message_type(
                ash::vk::DebugUtilsMessageTypeFlagsEXT::GENERAL
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::VALIDATION
                    | ash::vk::DebugUtilsMessageTypeFlagsEXT::PERFORMANCE
            ).pfn_user_callback(Some(vulkan_debug_callback));

        let debug_report = ash::extensions::ext::DebugUtils::new(entry, instance);

        let debug_report_callback = unsafe {
            debug_report.create_debug_utils_messenger(&create_info, None)?
        };

        Ok((debug_report, debug_report_callback))
    }

    fn check_validation_layer_support(
        entry: &ash::Entry
    ) -> anyhow::Result<bool, RenderError> {
        let layer_properties = entry
            .enumerate_instance_layer_properties()?;

        if layer_properties.len() <= 0 {
            log::error!("No available layers");

            return Ok(false);
        } else {
            log::debug!("Instance available layers: ");

            for layer in layer_properties.iter() {
                let layer_name = unsafe {
                    CStr::from_ptr(layer.layer_name.as_ptr())
                };

                let layer_name = layer_name
                    .to_str()
                    .unwrap();

                log::debug!("\t{}", layer_name);
            }
        }

        for required_layer_name in VALIDATION_LAYERS.iter() {
            let mut found = false;

            for layer_property in layer_properties.iter() {
                let layer_name = unsafe {
                    CStr::from_ptr(layer_property.layer_name.as_ptr())
                };

                let layer_name = layer_name
                    .to_str()
                    .unwrap();

                if (*required_layer_name) == layer_name {
                    found = true;
                    break;
                }
            }

            if found == false {
                return Ok(false);
            }
        }

        Ok(true)
    }

    pub fn get_enabled_layers() -> (Vec<CString>, Vec<*const i8>) {
        let layer_names = VALIDATION_LAYERS
            .iter()
            .map(|name| CString::new(*name).expect("Failed to build CString"))
            .collect::<Vec<_>>();

        let layer_names_ptrs = layer_names
            .iter()
            .map(|name| name.as_ptr())
            .collect::<Vec<_>>();

        (layer_names, layer_names_ptrs)
    }

    fn get_required_extensions() -> Vec<*const i8> {
        let mut extensions = Vec::new();

        extensions.push(ash::extensions::khr::Surface::name().as_ptr());

        #[cfg(target_os="windows")]
        extensions.push(ash::extensions::khr::Win32Surface::name().as_ptr());

        #[cfg(target_os="linux")]
        extensions.push(ash::extensions::khr::XlibSurface::name().as_ptr());

        if ENABLE_VALIDATION_LAYERS {
            extensions.push(ash::extensions::ext::DebugUtils::name().as_ptr());
        }

        log::debug!("Number of required extensions: {}", extensions.len());

        extensions
    }

    #[inline]
    pub fn inner(&self) -> &ash::Instance {
        &self.instance
    }
}

impl Drop for Instance {
    fn drop(&mut self) {
        log::debug!("Dropping vulkan instance");

        unsafe {
            if let Some((report, callback)) = self.debug_messenger.take() {
                report.destroy_debug_utils_messenger(callback, None);
            }

            self.instance.destroy_instance(None);
        }
    }
}
