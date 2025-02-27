#![allow(unused_variables)]
#![allow(dead_code)]
mod debug;
mod assets;
use assets::damn;
use assets::convert;
mod device;
mod surface;
mod swapchain;
mod instance;
mod physical_device;
mod queue;
mod input;
mod movement;


use bytemuck::Pod;
use bytemuck::Zeroable;
use input::Input;
use movement::Movement;
use winit::keyboard::KeyCode;
use std::collections::HashMap;
use std::time::Instant;
use ash;
use ash::vk;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::{Window, WindowId};

#[repr(C)]
#[derive(Copy, Clone, Pod, Zeroable)]
struct PushConstants {
    screen_resolution: vek::Vec2<f32>,
    _padding: vek::Vec2<f32>,
    matrix: vek::Mat4<f32>,
    position: vek::Vec4<f32>,
}

struct InternalApp {
    input: Input,
    movement: Movement,

    window: Window,
    entry: ash::Entry,
    device: ash::Device,
    instance: ash::Instance,
    physical_device: vk::PhysicalDevice,
    debug_messenger: Option<(ash::ext::debug_utils::Instance, vk::DebugUtilsMessengerEXT)>,
    surface_loader: ash::khr::surface::Instance,
    surface_khr: vk::SurfaceKHR,
    swapchain_loader: ash::khr::swapchain::Device,
    swapchain: vk::SwapchainKHR,
    images: Vec<vk::Image>,
    begin_semaphore: vk::Semaphore,
    end_semaphore: vk::Semaphore,
    end_fence: vk::Fence,
    queue: vk::Queue,
    queue_family_index: u32,
    pool: vk::CommandPool,
    compute_shader_module: vk::ShaderModule,
    descriptor_set_layout: vk::DescriptorSetLayout,
    pipeline_layout: vk::PipelineLayout,
    compute_pipeline: vk::Pipeline,
    descriptor_pool: vk::DescriptorPool,
    allocator: gpu_allocator::vulkan::Allocator,
    voxel_image: vk::Image,
    voxel_image_view: vk::ImageView,
    voxel_image_allocation: gpu_allocator::vulkan::Allocation,
}

impl InternalApp {
    pub unsafe fn new(event_loop: &ActiveEventLoop) -> Self {
        let mut assets = HashMap::<&str, Vec<u32>>::new();
        asset!("test.spv", assets);
        let raw = &*assets["test.spv"];

        let window = event_loop.create_window(Window::default_attributes()).unwrap();
        window.set_cursor_grab(winit::window::CursorGrabMode::Confined).unwrap();
        window.set_cursor_visible(false);
        let raw_display_handle = window.display_handle().unwrap().as_raw();        
        let entry = ash::Entry::load().unwrap();
        
        let instance = instance::create_instance(&entry, raw_display_handle);
        log::info!("created instance");
        let debug_messenger = debug::create_debug_messenger(&entry, &instance);  
        log::info!("created debug utils messenger");      
        let (surface_loader, surface_khr) = surface::create_surface(&instance, &entry, &window);
        log::info!("created surface");      

        let mut physical_device_candidates =  instance.enumerate_physical_devices().unwrap().into_iter().map(|physical_device| {
            let score = physical_device::get_physical_device_score(physical_device, &instance, &surface_loader, surface_khr);
            (physical_device, score)
        }).filter_map(|(a, b)| b.map(|val| (a, val))).collect::<Vec<(vk::PhysicalDevice, u32)>>();
        physical_device_candidates.sort_by(|(_, a), (_, b)| a.cmp(b));
        let physical_device = physical_device_candidates[0].0;
        log::info!("selected physical device");
        
        let (device, queue_family_index, queue) = device::create_device_and_queue(&instance, physical_device, &surface_loader, surface_khr);
        log::info!("created device and fetched main queue");

        let pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let pool = device.create_command_pool(&pool_create_info, None).unwrap();
        log::info!("create cmd pool");
        
        let (swapchain_loader, swapchain, images) = swapchain::create_swapchain(&instance, &surface_loader, surface_khr, physical_device, &device, vk::Extent2D {
            width: 800,
            height: 600,
        });
        let begin_semaphore = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap();
        let end_semaphore = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap();
        let end_fence = device.create_fence(&Default::default(), None).unwrap();

        let compute_shader_module_create_info = vk::ShaderModuleCreateInfo::default()
            .code(raw)
            .flags(vk::ShaderModuleCreateFlags::empty());
        let compute_shader_module = device.create_shader_module(&compute_shader_module_create_info, None).unwrap();

        let compute_stage_create_info = vk::PipelineShaderStageCreateInfo::default()
            .flags(vk::PipelineShaderStageCreateFlags::empty())
            .name(c"main")
            .stage(vk::ShaderStageFlags::COMPUTE)
            .module(compute_shader_module);

        let descriptor_set_layout_binding = vk::DescriptorSetLayoutBinding::default()
            .binding(0)
            .stage_flags(vk::ShaderStageFlags::COMPUTE)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .descriptor_count(1);
        let descriptor_set_layout_bindings = [descriptor_set_layout_binding];

        let descriptor_set_layout_create_info = vk::DescriptorSetLayoutCreateInfo::default()
            .flags(vk::DescriptorSetLayoutCreateFlags::empty())
            .bindings(&descriptor_set_layout_bindings);

        let descriptor_set_layout = device.create_descriptor_set_layout(&descriptor_set_layout_create_info, None).unwrap();
        let descriptor_set_layouts = [descriptor_set_layout];

        let push_constant_range = vk::PushConstantRange::default()
            .offset(0)
            .size(size_of::<PushConstants>() as u32)
            .stage_flags(vk::ShaderStageFlags::COMPUTE);
        let push_constants = [push_constant_range];

        let pipeline_layout_create_info = vk::PipelineLayoutCreateInfo::default()
            .push_constant_ranges(&push_constants)
            .flags(vk::PipelineLayoutCreateFlags::empty())
            .set_layouts(&descriptor_set_layouts);

        let pipeline_layout = device.create_pipeline_layout(&pipeline_layout_create_info, None).unwrap();

        let compute_pipeline_create_info = vk::ComputePipelineCreateInfo::default()
            .layout(pipeline_layout)
            .stage(compute_stage_create_info);
        let compute_pipelines = device.create_compute_pipelines(vk::PipelineCache::null(), &[compute_pipeline_create_info], None).unwrap();
        let compute_pipeline = compute_pipelines[0];

        let descriptor_pool_size = vk::DescriptorPoolSize::default()
            .descriptor_count(1)
            .ty(vk::DescriptorType::STORAGE_IMAGE);
        let descriptor_pool_sizes = [descriptor_pool_size]; 

        let descriptor_pool_create_info = vk::DescriptorPoolCreateInfo::default()
            .flags(vk::DescriptorPoolCreateFlags::FREE_DESCRIPTOR_SET)
            .max_sets(1)
            .pool_sizes(&descriptor_pool_sizes);

        let descriptor_pool = device.create_descriptor_pool(&descriptor_pool_create_info, None).unwrap();

        const SIZE: u32 = 256;
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
        
        let mut allocator = gpu_allocator::vulkan::Allocator::new(&gpu_allocator::vulkan::AllocatorCreateDesc {
            instance: instance.clone(),
            device: device.clone(),
            physical_device: physical_device.clone(),
            debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
            buffer_device_address: false,
            allocation_sizes: gpu_allocator::AllocationSizes::default(),
        }).unwrap();

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
            .format(vk::Format::R8_UINT)
            .view_type(vk::ImageViewType::TYPE_3D)
            .subresource_range(subresource_range);
        let voxel_image_view = device.create_image_view(&voxel_image_view_create_info, None).unwrap();

        Self {
            input: Default::default(),
            movement: Movement {
                position: vek::Vec3::unit_y() * 3f32,
                ..Default::default()
            },
            window,
            instance,
            entry,
            device,
            physical_device,
            surface_loader,
            surface_khr,
            debug_messenger,
            swapchain_loader,
            swapchain,
            images,
            begin_semaphore,
            end_semaphore,
            queue_family_index,
            end_fence,
            queue,
            pool,
            compute_shader_module,
            descriptor_set_layout,
            pipeline_layout,
            compute_pipeline,
            descriptor_pool,
            allocator,
            voxel_image,
            voxel_image_view,
            voxel_image_allocation: allocation,
        }
    }

    pub unsafe fn resize(&mut self, width: u32, height: u32) {
        self.device.device_wait_idle().unwrap();

        self.swapchain_loader.destroy_swapchain(self.swapchain, None);

        let extent = vk::Extent2D {
            width,
            height
        };

        let (swapchain_loader, swapchain, images) = swapchain::create_swapchain(&self.instance, &self.surface_loader, self.surface_khr, self.physical_device, &self.device, extent);
        self.images = images;
        self.swapchain_loader = swapchain_loader;
        self.swapchain = swapchain;
    }

    pub unsafe fn render(&mut self, delta: f32, elapsed: f32,) {
        self.device.reset_fences(&[self.end_fence]).unwrap();

        let (index, _) = self.swapchain_loader.acquire_next_image(
            self.swapchain,
            u64::MAX,
            self.begin_semaphore,
            vk::Fence::null()
        ).unwrap();
        let image = self.images[index as usize];

        let cmd_buffer_create_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.pool);
        let cmd = self.device.allocate_command_buffers(&cmd_buffer_create_info).unwrap()[0];

        let cmd_buffer_begin_info = vk::CommandBufferBeginInfo::default().flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        self.device.begin_command_buffer(cmd, &cmd_buffer_begin_info).unwrap();

        let subresource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);

        let image_view_create_info = vk::ImageViewCreateInfo::default()
            .components(vk::ComponentMapping::default())
            .flags(vk::ImageViewCreateFlags::empty())
            .format(vk::Format::R8G8B8A8_UNORM)
            .image(image)
            .subresource_range(subresource_range)
            .view_type(vk::ImageViewType::TYPE_2D);
        
        let image_view = self.device.create_image_view(&image_view_create_info, None).unwrap();


        let undefined_to_clear_layout_transition = vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::GENERAL)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE | vk::AccessFlags2::TRANSFER_WRITE)
            .src_stage_mask(vk::PipelineStageFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .src_queue_family_index(self.queue_family_index)
            .dst_queue_family_index(self.queue_family_index)
            .image(image)
            .subresource_range(subresource_range);
        let image_memory_barriers = [undefined_to_clear_layout_transition];
        let dep = vk::DependencyInfo::default()
            .image_memory_barriers(&image_memory_barriers);
        self.device.cmd_pipeline_barrier2(cmd, &dep);

        self.device.cmd_clear_color_image(cmd, image, vk::ImageLayout::GENERAL, &vk::ClearColorValue {
            float32: [elapsed.sin() * 0.5 + 0.5; 4]
        }, &[subresource_range]);

        let layouts = [self.descriptor_set_layout];
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts);
        let descriptor_sets = self.device.allocate_descriptor_sets(&descriptor_set_allocate_info).unwrap();
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

        self.device.update_descriptor_sets(&[descriptor_write], &[]);
        
        self.device.cmd_bind_descriptor_sets(cmd, vk::PipelineBindPoint::COMPUTE, self.pipeline_layout, 0, &descriptor_sets, &[]);
        self.device.cmd_bind_pipeline(cmd, vk::PipelineBindPoint::COMPUTE, self.compute_pipeline);

        let size = self.window.inner_size();
        let width_group_size = (size.width as f32 / 32f32).ceil() as u32;
        let height_group_size = (size.height as f32 / 32f32).ceil() as u32;

        let size = vek::Vec2::<u32>::new(size.width, size.height).map(|x| x as f32);

        let push_constants = PushConstants {
            screen_resolution: size,
            _padding: Default::default(),
            matrix: self.movement.proj_matrix * self.movement.view_matrix,
            position: self.movement.position.with_w(0f32),
        };

        let raw = bytemuck::bytes_of(&push_constants);

        self.device.cmd_push_constants(cmd, self.pipeline_layout, vk::ShaderStageFlags::COMPUTE, 0, raw);
        self.device.cmd_dispatch(cmd, width_group_size, height_group_size, 1);
        
        let clear_to_present_layout_transition = vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::GENERAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_access_mask(vk::AccessFlags2::SHADER_STORAGE_WRITE | vk::AccessFlags2::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_stage_mask(vk::PipelineStageFlags2::NONE)
            .src_queue_family_index(self.queue_family_index)
            .dst_queue_family_index(self.queue_family_index)
            .image(image)
            .subresource_range(subresource_range);
        let image_memory_barriers = [clear_to_present_layout_transition];
        let dep = vk::DependencyInfo::default()
            .image_memory_barriers(&image_memory_barriers);
        self.device.cmd_pipeline_barrier2(cmd, &dep);
        

        self.device.end_command_buffer(cmd).unwrap();

        let cmds = [cmd];
        let rendered_semaphores = [self.end_semaphore];
        let acquire_sempahores = [self.begin_semaphore];
        let wait_masks = [vk::PipelineStageFlags::ALL_COMMANDS | vk::PipelineStageFlags::ALL_GRAPHICS];
        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&cmds)
            .signal_semaphores(&rendered_semaphores)
            .wait_dst_stage_mask(&wait_masks)
            .wait_semaphores(&acquire_sempahores);
        self.device.queue_submit(self.queue, &[submit_info], self.end_fence).unwrap();

        let swapchains = [self.swapchain];
        let indices = [index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .image_indices(&indices)
            .wait_semaphores(&rendered_semaphores);

        self.device.wait_for_fences(&[self.end_fence], true, u64::MAX).unwrap();
        self.swapchain_loader.queue_present(self.queue, &present_info).unwrap();
        self.device.free_command_buffers(self.pool, &[cmd]);

        self.device.destroy_image_view(image_view, None);
        self.device.free_descriptor_sets(self.descriptor_pool, &descriptor_sets).unwrap();
    }

    pub unsafe fn destroy(mut self) {
        self.device.destroy_pipeline(self.compute_pipeline, None);
        self.device.destroy_pipeline_layout(self.pipeline_layout, None);
        self.device.destroy_descriptor_set_layout(self.descriptor_set_layout, None);
        self.device.destroy_shader_module(self.compute_shader_module, None);

        self.device.destroy_descriptor_pool(self.descriptor_pool, None);

        self.device.destroy_image_view(self.voxel_image_view, None);
        self.device.destroy_image(self.voxel_image, None);
        self.allocator.free(self.voxel_image_allocation).unwrap();
        log::info!("destroyed voxel image");

        // TODO: Just cope with the error messages vro 
        self.device.wait_for_fences(&[self.end_fence], true, u64::MAX).unwrap();
        self.device.destroy_semaphore(self.begin_semaphore, None);
        self.device.destroy_semaphore(self.end_semaphore, None);
        self.device.destroy_fence(self.end_fence, None);
        self.swapchain_loader.destroy_swapchain(self.swapchain, None);
        log::info!("destroyed swapchain");

        self.surface_loader.destroy_surface(self.surface_khr, None);
        log::info!("destroyed surface");

        self.device.destroy_command_pool(self.pool, None);
        log::info!("destroyed cmd pool");

        self.device.destroy_device(None);
        log::info!("destroyed device");

        if let Some((inst, debug_messenger)) = self.debug_messenger {
            inst.destroy_debug_utils_messenger(debug_messenger, None);
            log::info!("destroyed debug utils messenger");
        }

        self.instance.destroy_instance(None);
        log::info!("destroyed instance");
    }
}

struct App {
    internal: Option<InternalApp>,
    start: Instant,
    last: Instant
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        unsafe {
            self.internal = Some(InternalApp::new(event_loop));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, _: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => unsafe {
                event_loop.exit();
                self.internal.take().unwrap().destroy();
            },
            WindowEvent::RedrawRequested => unsafe {
                let inner = self.internal.as_mut().unwrap();
                let new = Instant::now(); 
                let elapsed = (new - self.start).as_secs_f32();
                let delta  = (new - self.last).as_secs_f32();

                let size = inner.window.inner_size().cast::<f32>();
                inner.movement.update(&inner.input, size.width / size.height, delta);

                if inner.input.get_button(KeyCode::F5).pressed() {
                    if inner.window.fullscreen().is_none() {
                        inner.window.set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                    } else {
                        inner.window.set_fullscreen(None);
                    }
                }

                inner.window.request_redraw();
                inner.render(delta, elapsed);
                self.last = new;
                input::update(&mut inner.input);
            }
            WindowEvent::Resized(new) => unsafe {
                let inner = self.internal.as_mut().unwrap();
                inner.resize(new.width, new.height);
            }

            // This is horrid...
            _ => {
                let inner = self.internal.as_mut().unwrap();
                input::window_event(&mut inner.input, &event);
            },
        }
    }

    fn device_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            device_id: winit::event::DeviceId,
            event: winit::event::DeviceEvent,
        ) {
        let inner = self.internal.as_mut().unwrap();
        input::device_event(&mut inner.input, &event);
    }
}

pub fn main() {
    env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Debug).init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        start: Instant::now(),
        last: Instant::now(),
        internal: None,
    };
    event_loop.run_app(&mut app).unwrap();
}