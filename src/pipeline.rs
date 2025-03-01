use ash::vk;


pub unsafe fn create_compute_pipeline(raw: &[u32], device: &ash::Device, push_constants_size: u32) -> (vk::ShaderModule, vk::DescriptorSetLayout, vk::PipelineLayout, vk::Pipeline) {
    let render_compute_shader_module_create_info = vk::ShaderModuleCreateInfo::default()
        .code(raw)
        .flags(vk::ShaderModuleCreateFlags::empty());
    let render_compute_shader_module = device.create_shader_module(&render_compute_shader_module_create_info, None).unwrap();

    let render_compute_stage_create_info = vk::PipelineShaderStageCreateInfo::default()
        .flags(vk::PipelineShaderStageCreateFlags::empty())
        .name(c"main")
        .stage(vk::ShaderStageFlags::COMPUTE)
        .module(render_compute_shader_module);

    let render_descriptor_set_layout_binding_rt_image = vk::DescriptorSetLayoutBinding::default()
        .binding(0)
        .stage_flags(vk::ShaderStageFlags::COMPUTE)
        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
        .descriptor_count(1);
    let render_descriptor_set_layout_binding_voxel_image = vk::DescriptorSetLayoutBinding::default()
        .binding(1)
        .stage_flags(vk::ShaderStageFlags::COMPUTE)
        .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
        .descriptor_count(1);
    let render_descriptor_set_layout_bindings = [render_descriptor_set_layout_binding_rt_image,render_descriptor_set_layout_binding_voxel_image];

    let render_descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::default()
        .flags(vk::DescriptorSetLayoutCreateFlags::empty())
        .bindings(&render_descriptor_set_layout_bindings);

    let render_compute_descriptor_set_layout = device.create_descriptor_set_layout(&render_descriptor_set_layout_create_info, None).unwrap();
    let render_compute_descriptor_set_layouts = [render_compute_descriptor_set_layout];

    let render_push_constant_range = vk::PushConstantRange::default()
        .offset(0)
        .size(push_constants_size)
        .stage_flags(vk::ShaderStageFlags::COMPUTE);
    let render_push_constants = [render_push_constant_range];

    let render_compute_pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
        .push_constant_ranges(&render_push_constants)
        .flags(vk::PipelineLayoutCreateFlags::empty())
        .set_layouts(&render_compute_descriptor_set_layouts);

    let render_compute_pipeline_layout = device.create_pipeline_layout(&render_compute_pipeline_layout_create_info, None).unwrap();

    let render_compute_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
        .layout(render_compute_pipeline_layout)
        .stage(render_compute_stage_create_info);
    let render_compute_pipelines = device.create_compute_pipelines(vk::PipelineCache::null(), &[render_compute_pipeline_create_info], None).unwrap();
    let render_compute_pipeline = render_compute_pipelines[0];
    (render_compute_shader_module, render_compute_descriptor_set_layout, render_compute_pipeline_layout, render_compute_pipeline)
}