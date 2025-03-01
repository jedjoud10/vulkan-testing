use ash::vk;
use gpu_allocator::vulkan::{Allocation, Allocator};


pub unsafe fn create_voxel_image(device: &ash::Device, allocator: &mut Allocator) -> (vk::Image, Allocation, vk::ImageView) {
    const SIZE: u32 = 128;
    let voxel_image_create_info = vk::ImageCreateInfo::default()
        .extent(vk::Extent3D {
            width: SIZE,
            height: SIZE,
            depth: SIZE,
        })
        .format(vk::Format::R32_UINT)
        .image_type(vk::ImageType::TYPE_3D)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .mip_levels(1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .usage(vk::ImageUsageFlags::STORAGE)
        .samples(vk::SampleCountFlags::TYPE_1)
        .array_layers(1);
    let voxel_image = device.create_image(&voxel_image_create_info, None).unwrap();
    let requirements = device.get_image_memory_requirements(voxel_image);

    let allocation = allocator.allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
        name: "Voxel Image Allocation",
        requirements: requirements,
        linear: true,
        allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedImage(voxel_image),
        location: gpu_allocator::MemoryLocation::GpuOnly,
    }).unwrap();

    let device_memory = allocation.memory();

    device.bind_image_memory(voxel_image, device_memory, 0).unwrap();

    let subresource_range = vk::ImageSubresourceRange::default()
        .base_mip_level(0)
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_array_layer(0)
        .layer_count(1)
        .level_count(1);
    let voxel_image_view_create_info = vk::ImageViewCreateInfo::default()
        .image(voxel_image)
        .format(vk::Format::R32_UINT)
        .view_type(vk::ImageViewType::TYPE_3D)
        .subresource_range(subresource_range);
    let voxel_image_view = device.create_image_view(&voxel_image_view_create_info, None).unwrap();
    (voxel_image, allocation, voxel_image_view)
}