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
            .map(|(&physical_device, present_supported_per_family)| {
                // Get the features of the physical device
                let features = instance
                    .instance
                    .get_physical_device_features(physical_device);

                // Get the properties of the physical device
                let properties = instance
                    .instance
                    .get_physical_device_properties(physical_device);

                // Get the memory properties of the physical device
                let memory_properties = instance
                    .instance
                    .get_physical_device_memory_properties(
                        physical_device,
                    );

                // Get the surface capabilities of the physical device
                let surface_capabilities = surface.surface_loader
                    .get_physical_device_surface_capabilities(physical_device, surface.surface).unwrap();

                // Get the present modes of the physical device
                let present_modes = surface.surface_loader
                    .get_physical_device_surface_present_modes(physical_device, surface.surface).unwrap();
                    
                // Get the supported formats of the physical device
                let present_formats = surface.surface_loader
                    .get_physical_device_surface_formats(physical_device, surface.surface).unwrap();

                // Get the queue family properties of the physical device
                let queue_family_properties = instance
                    .instance
                    .get_physical_device_queue_family_properties(physical_device);

                // Check each device family and see if we can present to it
                let queue_family_surface_supported = 
                    present_supported_per_family.collect::<Vec<bool>>();

                // Get other properties from the adapter
                let name = CStr::from_ptr(properties.device_name.as_ptr()).to_str().unwrap().to_owned();

                // Convert the values to a simple adapter
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
            }).find(|adapter| adapter.is_physical_device_suitable_base())
            .expect("Could not find a suitable GPU to use!");

        log::debug!("Using the adpater {:?}", adapter.name);
        adapter
    }

    // Check wether or not a physical device is suitable for rendering
    // This checks the minimum requirements that we need to achieve to be able to render
    unsafe fn is_physical_device_suitable_base(
        &self,
    ) -> bool {
        use vk::PhysicalDeviceType;
        let _type = self.properties.device_type;
        let surface = self.surface_capabilities;
        let modes = self.present_modes.as_slice();
        let formats = self.present_formats.as_slice();
        log::debug!("Checking if adapter {} is suitable...", self.name);

        // Check if double buffering is supported
        let double_buffering_supported = surface.min_image_count == 2;
        log::debug!(
            "Adapter Double Buffering: {}",
            double_buffering_supported
        );

        // Check if the present format is supported
        let format_supported = formats
            .iter()
            .find(|format| {
                let format_ =
                    format.format == vk::Format::B8G8R8A8_SRGB;
                let color_space_ = format.color_space
                    == vk::ColorSpaceKHR::SRGB_NONLINEAR;
                format_ && color_space_
            })
            .is_some();
        log::debug!(
            "Adapter Swapchain Format Supported: {}",
            format_supported
        );

        // Check if the minimum required present mode is supported
        let present_supported = modes
            .iter()
            .find(|&&present| {
                let relaxed =
                    present == vk::PresentModeKHR::FIFO_RELAXED;
                let immediate =
                    present == vk::PresentModeKHR::IMMEDIATE;
                relaxed || immediate
            })
            .is_some();

        // Check the device type
        let device_type_okay = _type == PhysicalDeviceType::DISCRETE_GPU;
        log::debug!("Adapter Device Type: {:?}", _type);

        // All the checks must pass
        double_buffering_supported
            && format_supported
            && present_supported
            && device_type_okay
    }
}