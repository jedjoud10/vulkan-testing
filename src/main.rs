#![allow(unused_variables)]
#![allow(dead_code)]
mod assets;
mod debug;
use assets::convert;
use assets::damn;
mod device;
mod input;
mod instance;
mod movement;
mod physical_device;
mod pipeline;
mod pool;
mod queue;
mod surface;
mod swapchain;
mod voxel;

use ash;
use ash::vk;
use gpu_allocator::vulkan::Allocation;
use input::Input;
use movement::Movement;
use std::collections::HashMap;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::keyboard::KeyCode;
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::{Window, WindowId};

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
    rt_images: Vec<(vk::Image, Allocation)>,
    begin_semaphore: vk::Semaphore,
    end_semaphore: vk::Semaphore,
    end_fence: vk::Fence,
    queue: vk::Queue,
    queue_family_index: u32,
    pool: vk::CommandPool,
    render_compute_shader_module: vk::ShaderModule,
    render_compute_descriptor_set_layout: vk::DescriptorSetLayout,
    render_compute_pipeline_layout: vk::PipelineLayout,
    render_compute_pipeline: vk::Pipeline,

    voxel_compute_shader_module: vk::ShaderModule,
    voxel_compute_descriptor_set_layout: vk::DescriptorSetLayout,
    voxel_compute_pipeline_layout: vk::PipelineLayout,
    voxel_compute_pipeline: vk::Pipeline,

    descriptor_pool: vk::DescriptorPool,
    allocator: gpu_allocator::vulkan::Allocator,
    voxel_image: vk::Image,
    voxel_image_view: vk::ImageView,
    voxel_image_allocation: Allocation,
}

impl InternalApp {
    pub unsafe fn new(event_loop: &ActiveEventLoop) -> Self {
        let mut assets = HashMap::<&str, Vec<u32>>::new();
        asset!("raymarcher.spv", assets);
        asset!("voxel.spv", assets);

        let window = event_loop
            .create_window(Window::default_attributes())
            .unwrap();
        window
            .set_cursor_grab(winit::window::CursorGrabMode::Confined)
            .unwrap();
        window.set_cursor_visible(false);
        let raw_display_handle = window.display_handle().unwrap().as_raw();
        let entry = ash::Entry::load().unwrap();

        let instance = instance::create_instance(&entry, raw_display_handle);
        log::info!("created instance");
        let debug_messenger = debug::create_debug_messenger(&entry, &instance);
        log::info!("created debug utils messenger");
        let (surface_loader, surface_khr) = surface::create_surface(&instance, &entry, &window);
        log::info!("created surface");

        let mut physical_device_candidates = instance
            .enumerate_physical_devices()
            .unwrap()
            .into_iter()
            .map(|physical_device| {
                let score = physical_device::get_physical_device_score(
                    physical_device,
                    &instance,
                    &surface_loader,
                    surface_khr,
                );
                (physical_device, score)
            })
            .filter_map(|(a, b)| b.map(|val| (a, val)))
            .collect::<Vec<(vk::PhysicalDevice, u32)>>();
        physical_device_candidates.sort_by(|(_, a), (_, b)| a.cmp(b));
        let physical_device = physical_device_candidates[0].0;
        log::info!("selected physical device");

        let (device, queue_family_index, queue) = device::create_device_and_queue(
            &instance,
            physical_device,
            &surface_loader,
            surface_khr,
        );
        let queue_family_indices = [queue_family_index];
        log::info!("created device and fetched main queue");

        let mut allocator =
            gpu_allocator::vulkan::Allocator::new(&gpu_allocator::vulkan::AllocatorCreateDesc {
                instance: instance.clone(),
                device: device.clone(),
                physical_device: physical_device.clone(),
                debug_settings: gpu_allocator::AllocatorDebugSettings::default(),
                buffer_device_address: false,
                allocation_sizes: gpu_allocator::AllocationSizes::default(),
            })
            .unwrap();
        log::info!("created gpu allocator");

        let pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let pool = device.create_command_pool(&pool_create_info, None).unwrap();
        log::info!("create cmd pool");

        let extent = vk::Extent2D {
            width: 800,
            height: 600,
        };

        let (swapchain_loader, swapchain, images) = swapchain::create_swapchain(
            &instance,
            &surface_loader,
            surface_khr,
            physical_device,
            &device,
            extent,
        );
        log::info!("created swapchain with {} in-flight images", images.len());

        let rt_images: Vec<(vk::Image, Allocation)> = (0..images.len())
            .into_iter()
            .map(|_| {
                swapchain::create_temporary_target_render_texture(
                    &instance,
                    &surface_loader,
                    surface_khr,
                    physical_device,
                    &device,
                    &mut allocator,
                    queue_family_index,
                    extent,
                )
            })
            .collect();
        log::info!("created {} in-flight render texture images", images.len());

        swapchain::transfer_rt_images(&device, queue_family_index, &rt_images, pool, queue);
        log::info!("transferred layout of render texture images");

        let begin_semaphore = device
            .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            .unwrap();
        let end_semaphore = device
            .create_semaphore(&vk::SemaphoreCreateInfo::default(), None)
            .unwrap();
        let end_fence = device.create_fence(&Default::default(), None).unwrap();

        let descriptor_pool = pool::create_descriptor_pool(&device);

        let (
            render_compute_shader_module,
            render_compute_descriptor_set_layout,
            render_compute_pipeline_layout,
            render_compute_pipeline,
        ) = pipeline::create_render_compute_pipeline(&*assets["raymarcher.spv"], &device);
        log::info!("created render compute pipeline");

        let (
            voxel_compute_shader_module,
            voxel_compute_descriptor_set_layout,
            voxel_compute_pipeline_layout,
            voxel_compute_pipeline,
        ) = pipeline::create_compute_voxel_pipeline(&*assets["voxel.spv"], &device);
        log::info!("created voxel compute pipeline");

        let (voxel_image, allocation, voxel_image_view) =
            voxel::create_voxel_image(&device, &mut allocator);

        voxel::generate_voxel_image(
            &device,
            queue,
            pool,
            descriptor_pool,
            queue_family_index,
            voxel_image,
            voxel_image_view,
            voxel_compute_descriptor_set_layout,
            voxel_compute_pipeline_layout,
            voxel_compute_pipeline
        );

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
            render_compute_shader_module,
            render_compute_descriptor_set_layout,
            render_compute_pipeline_layout,
            render_compute_pipeline,
            voxel_compute_shader_module,
            voxel_compute_descriptor_set_layout,
            voxel_compute_pipeline_layout,
            voxel_compute_pipeline,
            descriptor_pool,
            allocator,
            voxel_image,
            voxel_image_view,
            voxel_image_allocation: allocation,
            rt_images,
        }
    }

    pub unsafe fn resize(&mut self, width: u32, height: u32) {
        self.device.device_wait_idle().unwrap();

        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);

        for (image, allocation) in self.rt_images.drain(..) {
            self.device.destroy_image(image, None);
            self.allocator.free(allocation).unwrap();
        }

        let extent = vk::Extent2D { width, height };

        let (swapchain_loader, swapchain, images) = swapchain::create_swapchain(
            &self.instance,
            &self.surface_loader,
            self.surface_khr,
            self.physical_device,
            &self.device,
            extent,
        );
        self.images = images;
        self.swapchain_loader = swapchain_loader;
        self.swapchain = swapchain;

        let rt_images: Vec<(vk::Image, Allocation)> = (0..self.images.len())
            .into_iter()
            .map(|_| {
                swapchain::create_temporary_target_render_texture(
                    &self.instance,
                    &self.surface_loader,
                    self.surface_khr,
                    self.physical_device,
                    &self.device,
                    &mut self.allocator,
                    self.queue_family_index,
                    extent,
                )
            })
            .collect();
        swapchain::transfer_rt_images(
            &self.device,
            self.queue_family_index,
            &rt_images,
            self.pool,
            self.queue,
        );
        self.rt_images = rt_images;
    }

    pub unsafe fn render(&mut self, delta: f32, elapsed: f32) {
        self.device.reset_fences(&[self.end_fence]).unwrap();

        let (index, _) = self
            .swapchain_loader
            .acquire_next_image(
                self.swapchain,
                u64::MAX,
                self.begin_semaphore,
                vk::Fence::null(),
            )
            .unwrap();
        let dst_image = self.images[index as usize];
        let (src_image, _) = self.rt_images[index as usize];

        let cmd_buffer_create_info = vk::CommandBufferAllocateInfo::default()
            .command_buffer_count(1)
            .level(vk::CommandBufferLevel::PRIMARY)
            .command_pool(self.pool);
        let cmd = self
            .device
            .allocate_command_buffers(&cmd_buffer_create_info)
            .unwrap()[0];

        let cmd_buffer_begin_info = vk::CommandBufferBeginInfo::default()
            .flags(vk::CommandBufferUsageFlags::ONE_TIME_SUBMIT);
        self.device
            .begin_command_buffer(cmd, &cmd_buffer_begin_info)
            .unwrap();

        let subresource_range = vk::ImageSubresourceRange::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .level_count(1)
            .layer_count(1);

        let src_image_view_create_info = vk::ImageViewCreateInfo::default()
            .components(vk::ComponentMapping::default())
            .flags(vk::ImageViewCreateFlags::empty())
            .format(vk::Format::R8G8B8A8_UNORM)
            .image(src_image)
            .subresource_range(subresource_range)
            .view_type(vk::ImageViewType::TYPE_2D);

        let dst_image_view_create_info = vk::ImageViewCreateInfo::default()
            .components(vk::ComponentMapping::default())
            .flags(vk::ImageViewCreateFlags::empty())
            .format(vk::Format::R8G8B8A8_UNORM)
            .image(dst_image)
            .subresource_range(subresource_range)
            .view_type(vk::ImageViewType::TYPE_2D);

        let src_image_view = self
            .device
            .create_image_view(&src_image_view_create_info, None)
            .unwrap();
        let dst_image_view = self
            .device
            .create_image_view(&dst_image_view_create_info, None)
            .unwrap();

        let dst_undefined_to_blit_dst_layout_transition = vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .src_stage_mask(vk::PipelineStageFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .src_queue_family_index(self.queue_family_index)
            .dst_queue_family_index(self.queue_family_index)
            .image(dst_image)
            .subresource_range(subresource_range);
        let image_memory_barriers = [dst_undefined_to_blit_dst_layout_transition];
        let dep = vk::DependencyInfo::default().image_memory_barriers(&image_memory_barriers);
        self.device.cmd_pipeline_barrier2(cmd, &dep);

        /*
        self.device.cmd_clear_color_image(cmd, dst_image, vk::ImageLayout::GENERAL, &vk::ClearColorValue {
            float32: [elapsed.sin() * 0.5 + 0.5; 4]
            }, &[subresource_range]);
        */

        let layouts = [self.render_compute_descriptor_set_layout];
        let descriptor_set_allocate_info = vk::DescriptorSetAllocateInfo::default()
            .descriptor_pool(self.descriptor_pool)
            .set_layouts(&layouts);
        let descriptor_sets = self
            .device
            .allocate_descriptor_sets(&descriptor_set_allocate_info)
            .unwrap();
        let descriptor_set = descriptor_sets[0];

        let descriptor_rt_image_info = vk::DescriptorImageInfo::default()
            .image_view(src_image_view)
            .image_layout(vk::ImageLayout::GENERAL)
            .sampler(vk::Sampler::null());
        let descriptor_voxel_image_info = vk::DescriptorImageInfo::default()
            .image_view(self.voxel_image_view)
            .image_layout(vk::ImageLayout::GENERAL)
            .sampler(vk::Sampler::null());
        let descriptor_rt_image_infos = [descriptor_rt_image_info];
        let descriptor_voxel_image_infos = [descriptor_voxel_image_info];

        let first_descriptor_write = vk::WriteDescriptorSet::default()
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .dst_binding(0)
            .dst_set(descriptor_set)
            .image_info(&descriptor_rt_image_infos);
        let second_descriptor_write = vk::WriteDescriptorSet::default()
            .descriptor_count(1)
            .descriptor_type(vk::DescriptorType::STORAGE_IMAGE)
            .dst_binding(1)
            .dst_set(descriptor_set)
            .image_info(&descriptor_voxel_image_infos);

        self.device
            .update_descriptor_sets(&[first_descriptor_write, second_descriptor_write], &[]);

        self.device.cmd_bind_descriptor_sets(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            self.render_compute_pipeline_layout,
            0,
            &descriptor_sets,
            &[],
        );
        self.device.cmd_bind_pipeline(
            cmd,
            vk::PipelineBindPoint::COMPUTE,
            self.render_compute_pipeline,
        );

        let size = self.window.inner_size();
        let size = vek::Vec2::<u32>::new(size.width, size.height)
            .map(|val| val / swapchain::SCALING_FACTOR);

        let width_group_size = (size.x as f32 / 32f32).ceil() as u32;
        let height_group_size = (size.y as f32 / 32f32).ceil() as u32;

        let size = size.map(|x| x as f32);

        let push_constants = pipeline::PushConstants {
            screen_resolution: size,
            _padding: Default::default(),
            matrix: self.movement.proj_matrix * self.movement.view_matrix,
            position: self.movement.position.with_w(0f32),
        };

        let raw = bytemuck::bytes_of(&push_constants);

        self.device.cmd_push_constants(
            cmd,
            self.render_compute_pipeline_layout,
            vk::ShaderStageFlags::COMPUTE,
            0,
            raw,
        );
        self.device
            .cmd_dispatch(cmd, width_group_size, height_group_size, 1);

        let origin_offset = vk::Offset3D::default();
        let src_extent_offset = vk::Offset3D::default()
            .x(self.window.inner_size().width as i32 / swapchain::SCALING_FACTOR as i32)
            .y(self.window.inner_size().height as i32 / swapchain::SCALING_FACTOR as i32)
            .z(1);
        let dst_extent_offset = vk::Offset3D::default()
            .x(self.window.inner_size().width as i32)
            .y(self.window.inner_size().height as i32)
            .z(1);
        let src_offsets = [origin_offset, src_extent_offset];
        let dst_offsets = [origin_offset, dst_extent_offset];

        let subresource_layers = vk::ImageSubresourceLayers::default()
            .aspect_mask(vk::ImageAspectFlags::COLOR)
            .base_array_layer(0)
            .layer_count(1)
            .mip_level(0);

        let image_blit = vk::ImageBlit::default()
            .src_offsets(src_offsets)
            .src_subresource(subresource_layers)
            .dst_offsets(dst_offsets)
            .dst_subresource(subresource_layers);

        let regions = [image_blit];
        self.device.cmd_blit_image(
            cmd,
            src_image,
            vk::ImageLayout::GENERAL,
            dst_image,
            vk::ImageLayout::TRANSFER_DST_OPTIMAL,
            &regions,
            vk::Filter::NEAREST,
        );

        let blit_dst_to_present_layout_transition = vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
            .src_stage_mask(vk::PipelineStageFlags2::ALL_COMMANDS)
            .dst_stage_mask(vk::PipelineStageFlags2::NONE)
            .src_queue_family_index(self.queue_family_index)
            .dst_queue_family_index(self.queue_family_index)
            .image(dst_image)
            .subresource_range(subresource_range);
        let image_memory_barriers = [blit_dst_to_present_layout_transition];
        let dep = vk::DependencyInfo::default().image_memory_barriers(&image_memory_barriers);
        self.device.cmd_pipeline_barrier2(cmd, &dep);

        self.device.end_command_buffer(cmd).unwrap();

        let cmds = [cmd];
        let rendered_semaphores = [self.end_semaphore];
        let acquire_sempahores = [self.begin_semaphore];
        let wait_masks =
            [vk::PipelineStageFlags::ALL_COMMANDS | vk::PipelineStageFlags::ALL_GRAPHICS];
        let submit_info = vk::SubmitInfo::default()
            .command_buffers(&cmds)
            .signal_semaphores(&rendered_semaphores)
            .wait_dst_stage_mask(&wait_masks)
            .wait_semaphores(&acquire_sempahores);
        self.device
            .queue_submit(self.queue, &[submit_info], self.end_fence)
            .unwrap();

        let swapchains = [self.swapchain];
        let indices = [index];
        let present_info = vk::PresentInfoKHR::default()
            .swapchains(&swapchains)
            .image_indices(&indices)
            .wait_semaphores(&rendered_semaphores);

        self.device
            .wait_for_fences(&[self.end_fence], true, u64::MAX)
            .unwrap();
        self.swapchain_loader
            .queue_present(self.queue, &present_info)
            .unwrap();
        self.device.free_command_buffers(self.pool, &[cmd]);

        self.device.destroy_image_view(src_image_view, None);
        self.device.destroy_image_view(dst_image_view, None);
        self.device
            .free_descriptor_sets(self.descriptor_pool, &descriptor_sets)
            .unwrap();
    }

    pub unsafe fn destroy(mut self) {
        pipeline::destroy(&self.device, self.render_compute_shader_module, self.render_compute_descriptor_set_layout, self.render_compute_pipeline_layout, self.render_compute_pipeline);
        pipeline::destroy(&self.device, self.voxel_compute_shader_module, self.voxel_compute_descriptor_set_layout, self.voxel_compute_pipeline_layout, self.voxel_compute_pipeline);
        
        self.device
            .destroy_descriptor_pool(self.descriptor_pool, None);
        log::info!("destroyed pipeline, layout, desc. set, shader module, and desc. pool");

        self.device.destroy_image_view(self.voxel_image_view, None);
        self.device.destroy_image(self.voxel_image, None);
        self.allocator.free(self.voxel_image_allocation).unwrap();
        log::info!("destroyed voxel image");

        // TODO: Just cope with the error messages vro
        self.device
            .wait_for_fences(&[self.end_fence], true, u64::MAX)
            .unwrap();
        self.device.destroy_semaphore(self.begin_semaphore, None);
        self.device.destroy_semaphore(self.end_semaphore, None);
        self.device.destroy_fence(self.end_fence, None);
        self.swapchain_loader
            .destroy_swapchain(self.swapchain, None);
        log::info!("destroyed swapchain");

        self.surface_loader.destroy_surface(self.surface_khr, None);
        log::info!("destroyed surface");

        for (image, allocation) in self.rt_images {
            self.device.destroy_image(image, None);
            self.allocator.free(allocation).unwrap();
        }
        log::info!("destroyed render target images");

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
    last: Instant,
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
                let delta = (new - self.last).as_secs_f32();

                let size = inner.window.inner_size().cast::<f32>();
                inner
                    .movement
                    .update(&inner.input, size.width / size.height, delta);

                if inner.input.get_button(KeyCode::F5).pressed() {
                    if inner.window.fullscreen().is_none() {
                        inner
                            .window
                            .set_fullscreen(Some(winit::window::Fullscreen::Borderless(None)));
                    } else {
                        inner.window.set_fullscreen(None);
                    }
                }

                inner.window.request_redraw();
                inner.render(delta, elapsed);
                self.last = new;
                input::update(&mut inner.input);
            },
            WindowEvent::Resized(new) => unsafe {
                let inner = self.internal.as_mut().unwrap();
                inner.resize(new.width, new.height);
            },

            // This is horrid...
            _ => {
                let inner = self.internal.as_mut().unwrap();
                input::window_event(&mut inner.input, &event);
            }
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
    env_logger::Builder::from_default_env()
        .filter_level(log::LevelFilter::Debug)
        .init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App {
        start: Instant::now(),
        last: Instant::now(),
        internal: None,
    };
    event_loop.run_app(&mut app).unwrap();
}
