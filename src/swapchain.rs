use ash::vk;


pub unsafe fn create_swapchain(
    instance: &ash::Instance,
    surface_loader: &ash::khr::surface::Instance,
    surface_khr: vk::SurfaceKHR,
    physical_device: vk::PhysicalDevice,
    device: &ash::Device,
    extent: vk::Extent2D,
) -> (ash::khr::swapchain::Device, vk::SwapchainKHR, Vec<vk::Image>) {
    let surface_capabilities =  surface_loader.get_physical_device_surface_capabilities(physical_device, surface_khr).unwrap();
    let present_modes: Vec<vk::PresentModeKHR> = surface_loader.get_physical_device_surface_present_modes(physical_device, surface_khr).unwrap();
    let surface_formats: Vec<vk::SurfaceFormatKHR> = surface_loader.get_physical_device_surface_formats(physical_device, surface_khr).unwrap();
    let present = present_modes.iter().copied().find(|&x| x == vk::PresentModeKHR::IMMEDIATE || x == vk::PresentModeKHR::MAILBOX).unwrap();
    let swapchain_create_info = vk::SwapchainCreateInfoKHR::default()
        .surface(surface_khr)
        .min_image_count(
            surface_capabilities.min_image_count,
        )
        .image_format(surface_formats[0].format)
        .image_color_space(surface_formats[0].color_space)
        .image_extent(extent)
        .image_array_layers(1)
        .pre_transform(vk::SurfaceTransformFlagsKHR::IDENTITY)
        .composite_alpha(vk::CompositeAlphaFlagsKHR::OPAQUE)
        .image_usage(
            vk::ImageUsageFlags::COLOR_ATTACHMENT
                | vk::ImageUsageFlags::TRANSFER_DST | vk::ImageUsageFlags::STORAGE,
        )
        .clipped(true)
        .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
        .old_swapchain(vk::SwapchainKHR::null())
        .present_mode(present);

    let swapchain_loader = ash::khr::swapchain::Device::new(instance, device);
    let swapchain = swapchain_loader.create_swapchain(&swapchain_create_info, None).unwrap();
    let images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
    (swapchain_loader, swapchain, images)
}