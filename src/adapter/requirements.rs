use ash::vk::{
    self, PhysicalDevice, PhysicalDeviceFeatures,
    PhysicalDeviceMemoryProperties, PhysicalDeviceProperties,
    PresentModeKHR, SurfaceCapabilitiesKHR, SurfaceFormatKHR,
};

use vk::PhysicalDeviceType;
use crate::{Instance, Surface};

impl super::Adapter {
    // Check wether or not a physical device is suitable for rendering
    // This checks the minimum requirements that we need to achieve to be able to render
    pub(super) unsafe fn is_physical_device_suitable(
        &self,
    ) -> bool {
        let _type = self.properties.device_type;
        let surface = self.surface_capabilities;
        let modes = self.present_modes.as_slice();
        let formats = self.present_formats.as_slice();
        log::debug!("Checking if adapter {} is suitable...", self.name);

        // Check all the requirements that are needed for us to use this Adapter
        let double_buffering_supported = is_double_buffering_supported(surface);
        let format_supported = is_surface_format_supported(formats);
        let present_supported = is_present_mode_supported(modes);
        let device_type_okay = is_device_type_optimal(_type);

        // All the checks must pass
        double_buffering_supported
            && format_supported
            && present_supported
            && device_type_okay
    }
}

// Check if the Adapter is optimal (dGPU)
fn is_device_type_optimal(_type: PhysicalDeviceType) -> bool {
    let device_type_okay = _type == PhysicalDeviceType::DISCRETE_GPU;
    log::debug!("Adapter Device Type: {:?}", _type);
    device_type_okay
}

// Check if the Adapter supports a min image count of 2
fn is_double_buffering_supported(surface: SurfaceCapabilitiesKHR) -> bool {
    let double_buffering_supported = surface.min_image_count == 2;
    log::debug!(
        "Adapter Double Buffering: {}",
        double_buffering_supported
    );
    double_buffering_supported
}

// Check if the Adapter present modes support FIFO_RELAXED and IMMEDIATE
fn is_present_mode_supported(modes: &[PresentModeKHR]) -> bool {
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
        
    present_supported
}

// Check if the Adapter formats supportB8G8R8A8_SRGB and SRGB_NONLINEAR
fn is_surface_format_supported(formats: &[SurfaceFormatKHR]) -> bool {
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
    format_supported
}