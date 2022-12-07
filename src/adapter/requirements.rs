use ash::vk::{
    self, PhysicalDevice, PhysicalDeviceFeatures,
    PhysicalDeviceMemoryProperties, PhysicalDeviceProperties,
    PresentModeKHR, SurfaceCapabilitiesKHR, SurfaceFormatKHR,
};

use crate::{Instance, Surface};

impl super::Adapter {
    // Check wether or not a physical device is suitable for rendering
    // This checks the minimum requirements that we need to achieve to be able to render
    pub(super) unsafe fn is_physical_device_suitable(
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