use ash::vk;
use gpu_allocator::vulkan::{Allocation, Allocator};

const SIZE: u32 = 128;

pub unsafe fn create_voxel_image(
    device: &ash::Device,
    allocator: &mut Allocator,
) -> (vk::Image, Allocation, vk::ImageView) {
    let voxel_image_create_info = vk::ImageCreateInfo::default()
        .extent(vk::Extent3D {
            width: SIZE,
            height: SIZE,
            depth: SIZE,
        })
        .format(vk::Format::R8_UINT)
        .image_type(vk::ImageType::TYPE_3D)
        .initial_layout(vk::ImageLayout::UNDEFINED)
        .mip_levels(1)
        .sharing_mode(vk::SharingMode::EXCLUSIVE)
        .usage(vk::ImageUsageFlags::STORAGE)
        .samples(vk::SampleCountFlags::TYPE_1)
        .array_layers(1);
    let voxel_image = device.create_image(&voxel_image_create_info, None).unwrap();
    let requirements = device.get_image_memory_requirements(voxel_image);

    let allocation = allocator
        .allocate(&gpu_allocator::vulkan::AllocationCreateDesc {
            name: "Voxel Image Allocation",
            requirements: requirements,
            linear: true,
            allocation_scheme: gpu_allocator::vulkan::AllocationScheme::DedicatedImage(voxel_image),
            location: gpu_allocator::MemoryLocation::GpuOnly,
        })
        .unwrap();

    let device_memory = allocation.memory();

    device
        .bind_image_memory(voxel_image, device_memory, 0)
        .unwrap();

    let subresource_range = vk::ImageSubresourceRange::default()
        .base_mip_level(0)
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_array_layer(0)
        .layer_count(1)
        .level_count(1);
    let voxel_image_view_create_info = vk::ImageViewCreateInfo::default()
        .image(voxel_image)
        .format(vk::Format::R8_UINT)
        .view_type(vk::ImageViewType::TYPE_3D)
        .subresource_range(subresource_range);
    let voxel_image_view = device
        .create_image_view(&voxel_image_view_create_info, None)
        .unwrap();
    (voxel_image, allocation, voxel_image_view)
}

pub unsafe fn generate_voxel_image(device: &ash::Device, queue: vk::Queue, pool: vk::CommandPool, descriptor_pool: vk::DescriptorPool, queue_family_index: u32, image: vk::Image, image_view: vk::ImageView, descriptor_set_layout: vk::DescriptorSetLayout, pipeline_layout: vk::PipelineLayout, pipeline: vk::Pipeline) {
    
    let cmd_buffer_create_info = vk::CommandBufferAllocateInfo::default()
        .command_buffer_count(1)
        .level(vk::CommandBufferLevel::PRIMARY)
        .command_pool(pool);
    let cmd = device
        .allocate_command_buffers(&cmd_buffer_create_info)
        .unwrap()[0];

    let cmd_buffer_begin_info = vk::CommandBufferBeginInfo::default()
        .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
    device
        .begin_command_buffer(cmd, &cmd_buffer_begin_info)
        .unwrap();

    let subresource_range = vk::ImageSubresourceRange::default()
        .base_mip_level(0)
        .aspect_mask(vk::ImageAspectFlags::COLOR)
        .base_array_layer(0)
        .layer_count(1)
        .level_count(1);

    let first_transition = vk::ImageMemoryBarrier2::default()
        .old_layout(vk::ImageLayout::UNDEFINED)
        .new_layout(vk::ImageLayout::GENERAL)
        .src_access_mask(vk::AccessFlags2::NONE)
        .dst_access_mask(vk::AccessFlags2::SHADER_WRITE)
        .src_stage_mask(vk::PipelineStageFlags2::NONE)
        .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .src_queue_family_index(queue_family_index)
        .dst_queue_family_index(queue_family_index)
        .image(image)
        .subresource_range(subresource_range);
    let image_memory_barriers = [first_transition];
    let dep = vk::DependencyInfo::default().image_memory_barriers(&image_memory_barriers);
    device.cmd_pipeline_barrier2(cmd, &dep);

    let layouts = [descriptor_set_layout];
    let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
        .descriptor_pool(descriptor_pool)
        .set_layouts(&layouts);
    let descriptor_sets = device
        .allocate_descriptor_sets(&descriptor_set_allocate_info)
        .unwrap();
    let descriptor_set = descriptor_sets[0];

    let descriptor_image_info = vk::DescriptorImageInfo::default()
        .image_view(image_view)
        .image_layout(vk::ImageLayout::GENERAL)
        .sampler(vk::Sampler::null());
    let descriptor_image_infos = [descriptor_image_info];

    let descriptor_write = vk::WriteDescriptorSet::default()
        .descriptor_count(1)
        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
        .dst_binding(0)
        .dst_set(descriptor_set)
        .image_info(&descriptor_image_infos);

    device
        .update_descriptor_sets(&[descriptor_write], &[]);

    device.cmd_bind_descriptor_sets(
        cmd,
        vk::PipelineBindPoint::COMPUTE,
        pipeline_layout,
        0,
        &descriptor_sets,
        &[],
    );

    device.cmd_bind_pipeline(
        cmd,
        vk::PipelineBindPoint::COMPUTE,
        pipeline,
    );

    device.cmd_dispatch(cmd, SIZE / 8, SIZE / 8, SIZE / 8);

    let second_transition = vk::ImageMemoryBarrier2::default()
        .old_layout(vk::ImageLayout::GENERAL)
        .new_layout(vk::ImageLayout::GENERAL)
        .src_access_mask(vk::AccessFlags2::SHADER_WRITE)
        .dst_access_mask(vk::AccessFlags2::SHADER_READ | vk::AccessFlags2::MEMORY_READ)
        .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
        .src_queue_family_index(queue_family_index)
        .dst_queue_family_index(queue_family_index)
        .image(image)
        .subresource_range(subresource_range);
    let image_memory_barriers = [second_transition];
    let dep = vk::DependencyInfo::default().image_memory_barriers(&image_memory_barriers);
    device.cmd_pipeline_barrier2(cmd, &dep);

    device.end_command_buffer(cmd).unwrap();

    let cmds = [cmd];
    let submit_info = vk::SubmitInfo::default()
        .command_buffers(&cmds)
        .signal_semaphores(&[])
        .wait_dst_stage_mask(&[])
        .wait_semaphores(&[]);

    let fence = device.create_fence(&Default::default(), None).unwrap();

    device.queue_submit(queue, &[submit_info], fence).unwrap();
    device.wait_for_fences(&[fence], true, u64::MAX).unwrap();
    device.free_command_buffers(pool, &[cmd]);
    device.free_descriptor_sets(descriptor_pool, &descriptor_sets).unwrap();
    device.destroy_fence(fence, None);
}