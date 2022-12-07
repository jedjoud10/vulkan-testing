use std::ffi::{CStr, CString};

use ash::vk::{
    self, PhysicalDevice, PhysicalDeviceFeatures,
    PhysicalDeviceMemoryProperties, PhysicalDeviceProperties,
    PresentModeKHR, SurfaceCapabilitiesKHR, SurfaceFormatKHR,
};

use crate::{Instance, Surface};

// An adapter is a physical device that was chosen manually by the user
// For now, this Vulkan abstraction library can only handle one adapter per instance
pub struct Adapter {
    // Raw physical device
    pub(crate) raw: PhysicalDevice,
    pub(crate) name: String,

    // Properties
    pub(crate) memory_properties: PhysicalDeviceMemoryProperties,
    pub(crate) features: PhysicalDeviceFeatures,
    pub(crate) properties: PhysicalDeviceProperties,
    pub(crate) surface_capabilities: SurfaceCapabilitiesKHR,

    // Swapchain related
    pub(crate) present_modes: Vec<PresentModeKHR>,
    pub(crate) present_formats: Vec<SurfaceFormatKHR>,

    // Related to queue families
    pub(crate) queue_family_properties:
        Vec<vk::QueueFamilyProperties>,
    pub(crate) queue_family_nums: usize,
    pub(crate) queue_family_surface_supported: Vec<bool>,
}

impl Adapter {
    // Wrap a raw physical device into an adapter
    unsafe fn wrap(
        instance: &Instance,
        physical_device: PhysicalDevice,
        surface: &Surface,
        present_supported_per_family: impl Iterator<Item = bool>
    ) -> Adapter {
        let features = instance
            .instance
            .get_physical_device_features(physical_device);
        let properties = instance
            .instance
            .get_physical_device_properties(physical_device);
        let memory_properties = instance
            .instance
            .get_physical_device_memory_properties(
                physical_device,
            );
        let surface_capabilities = surface.surface_loader
            .get_physical_device_surface_capabilities(physical_device, surface.surface).unwrap();
        let present_modes = surface.surface_loader
            .get_physical_device_surface_present_modes(physical_device, surface.surface).unwrap();
        let present_formats = surface.surface_loader
            .get_physical_device_surface_formats(physical_device, surface.surface).unwrap();
        let queue_family_properties = instance
            .instance
            .get_physical_device_queue_family_properties(physical_device);
        let queue_family_surface_supported = 
            present_supported_per_family.collect::<Vec<bool>>();
        let name = CStr::from_ptr(properties.device_name.as_ptr()).to_str().unwrap().to_owned();
        Adapter {
            raw: physical_device,
            name,
            memory_properties,
            features,
            properties,
            surface_capabilities,
            present_modes,
            present_formats,
            queue_family_nums: queue_family_properties.len(),
            queue_family_properties,
            queue_family_surface_supported,
        }
    }

    // Pick out a physical adapter automatically for the user
    // Pick a physical device from the Vulkan instance
    pub unsafe fn pick(
        instance: &Instance,
        surface: &Surface,
    ) -> Adapter {
        let devices =
            instance.instance.enumerate_physical_devices().unwrap();

        let adapter = devices
            .iter()
            .map(|physical_device| {
                // We must first check if the device supports the surface
                let len = instance
                    .instance
                    .get_physical_device_queue_family_properties(*physical_device)
                    .len();
                let range = 0..len;

                let present_supported = range.into_iter().map(|i| {
                    surface
                        .surface_loader
                        .get_physical_device_surface_support(
                            *physical_device, i as u32,
                            surface.surface
                        ).unwrap_or_default()
                });

                (physical_device, present_supported)
            })
            .map(|(&physical_device, present_supported_per_family)| 
                Self::wrap(instance, physical_device, surface, present_supported_per_family)).find(|adapter| Self::is_physical_device_suitable(adapter)
            )
            .expect("Could not find a suitable GPU to use!");

        log::debug!("Using the adpater {:?}", adapter.name);
        adapter
    }
}
