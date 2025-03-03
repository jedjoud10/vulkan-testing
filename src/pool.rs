use ash::vk;

pub unsafe fn create_descriptor_pool(device: &ash::Device) -> vk::DescriptorPool {
    let descriptor_pool_size = vk::DescriptorPoolSize::default()
        .descriptor_count(2)
        .ty(vk::DescriptorType::STORAGE_IMAGE);
    let descriptor_pool_sizes = [descriptor_pool_size];

    let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
        .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
        .max_sets(1)
        .pool_sizes(&descriptor_pool_sizes);

    let descriptor_pool = device
        .create_descriptor_pool(&descriptor_pool_create_info, None)
        .unwrap();
    descriptor_pool
}
