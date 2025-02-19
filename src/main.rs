use std::ffi::{CStr, CString};
use std::time::Instant;
mod debug;
mod surface;
mod physical_device;
mod queue;
use ash;
use ash::vk;
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, ControlFlow, EventLoop};
use winit::raw_window_handle::HasDisplayHandle;
use winit::window::{Window, WindowId};

struct InternalApp {
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
}

impl InternalApp {
    pub unsafe fn new(event_loop: &ActiveEventLoop) -> Self {
        let window = event_loop.create_window(Window::default_attributes()).unwrap();
        let rwh = window.display_handle().unwrap().as_raw();        
        let entry = ash::Entry::load().unwrap();

        let app_info = vk::ApplicationInfo::default()
            .application_name(c"")
            .api_version(vk::API_VERSION_1_3)
            .application_version(0)
            .engine_version(0)
            .engine_name(c"");

        let mut extension_names_ptrs =
            ash_window::enumerate_required_extensions(rwh)
            .unwrap()
            .to_vec();

        let required_instance_extensions = vec![
            ash::ext::debug_utils::NAME,
            ash::khr::surface::NAME,
        ];

        extension_names_ptrs.extend(
            required_instance_extensions.iter().map(|s| s.as_ptr()),
        );

        let required_validation_layers: Vec<&'static CStr> = {
            #[cfg(debug_assertions)]
            {vec![c"VK_LAYER_KHRONOS_validation"]}
            #[cfg(not(debug_assertions))]
            {vec![]}
        };

        let validation_ptrs = required_validation_layers
            .iter()
            .map(|cstr| cstr.as_ptr())
            .collect::<Vec<_>>();
        let instance_create_info = vk::InstanceCreateInfo::default()
            .application_info(&app_info)
            .enabled_layer_names(&validation_ptrs)
            .enabled_extension_names(&extension_names_ptrs);
        let instance = entry.create_instance(&instance_create_info, None).unwrap();
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
        
        let queue_family_properties = instance.get_physical_device_queue_family_properties(physical_device);
        let queue_family_index = queue::find_appropriate_queue_family_index(physical_device, queue_family_properties, &surface_loader, surface_khr) as u32;

        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_priorities(&[1.0])
            .queue_family_index(queue_family_index);
        let queue_create_infos = [queue_create_info];

        let device_features = vk::PhysicalDeviceFeatures::default();
        let mut device_features_13 = vk::PhysicalDeviceVulkan13Features::default()
            .synchronization2(true);

        let device_extension_names = vec![
            ash::khr::swapchain::NAME,
        ];

        let device_extension_names_ptrs = device_extension_names .iter()
            .map(|cstr| cstr.as_ptr())
            .collect::<Vec<_>>();

        let device_create_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&device_extension_names_ptrs)
            .enabled_features(&device_features)
            .queue_create_infos(&queue_create_infos)
            .push_next(&mut device_features_13);

        let device = instance.create_device(physical_device, &device_create_info, None).unwrap();
        log::info!("created device");   

        let queue = device.get_device_queue(queue_family_index, 0);
        log::info!("fetched queue");

        let pool_create_info = vk::CommandPoolCreateInfo::default()
            .queue_family_index(queue_family_index)
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER);
        let pool = device.create_command_pool(&pool_create_info, None).unwrap();
        log::info!("create cmd pool");
        
        let surface_capabilities =  surface_loader.get_physical_device_surface_capabilities(physical_device, surface_khr).unwrap();
        let present_modes: Vec<vk::PresentModeKHR> = surface_loader.get_physical_device_surface_present_modes(physical_device, surface_khr).unwrap();
        let surface_formats: Vec<vk::SurfaceFormatKHR> = surface_loader.get_physical_device_surface_formats(physical_device, surface_khr).unwrap();
        let present = present_modes.iter().copied().find(|&x| x == vk::PresentModeKHR::IMMEDIATE || x == vk::PresentModeKHR::MAILBOX).unwrap();
        let extent = surface_capabilities.current_extent;

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
                    | vk::ImageUsageFlags::TRANSFER_DST,
            )
            .clipped(true)
            .image_sharing_mode(vk::SharingMode::EXCLUSIVE)
            .old_swapchain(vk::SwapchainKHR::null())
            .present_mode(present);

        let swapchain_loader = ash::khr::swapchain::Device::new(&instance, &device);
        let swapchain = swapchain_loader.create_swapchain(&swapchain_create_info, None).unwrap();
        let images = swapchain_loader.get_swapchain_images(swapchain).unwrap();
        let begin_semaphore = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap();
        let end_semaphore = device.create_semaphore(&vk::SemaphoreCreateInfo::default(), None).unwrap();
        let end_fence = device.create_fence(&Default::default(), None).unwrap();


        Self {
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
        }
    }

    pub unsafe fn render(&mut self, delta: f32, elapsed: f32,) {
        self.device.reset_fences(&[self.end_fence]).unwrap();

        let (index, suboptimal) = self.swapchain_loader.acquire_next_image(
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

        let undefined_to_clear_layout_transition = vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::UNDEFINED)
            .new_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .src_access_mask(vk::AccessFlags2::NONE)
            .dst_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .src_stage_mask(vk::PipelineStageFlags2::NONE)
            .dst_stage_mask(vk::PipelineStageFlags2::ALL_TRANSFER)
            .src_queue_family_index(self.queue_family_index)
            .dst_queue_family_index(self.queue_family_index)
            .image(image)
            .subresource_range(subresource_range);
        let image_memory_barriers = [undefined_to_clear_layout_transition];
        let dep = vk::DependencyInfo::default()
            .image_memory_barriers(&image_memory_barriers);
        self.device.cmd_pipeline_barrier2(cmd, &dep);

        self.device.cmd_clear_color_image(cmd, image, vk::ImageLayout::TRANSFER_DST_OPTIMAL, &vk::ClearColorValue {
            float32: [elapsed.sin() * 0.5 + 0.5; 4]
        }, &[subresource_range]);

        let clear_to_present_layout_transition = vk::ImageMemoryBarrier2::default()
            .old_layout(vk::ImageLayout::TRANSFER_DST_OPTIMAL)
            .new_layout(vk::ImageLayout::PRESENT_SRC_KHR)
            .src_access_mask(vk::AccessFlags2::TRANSFER_WRITE)
            .dst_access_mask(vk::AccessFlags2::MEMORY_READ)
            .src_stage_mask(vk::PipelineStageFlags2::ALL_TRANSFER)
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
    }

    pub unsafe fn destroy(self) {
        if let Some((inst, debug_messenger)) = self.debug_messenger {
            inst.destroy_debug_utils_messenger(debug_messenger, None);
            log::info!("destroyed debug utils messenger");
        }

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

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
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
                inner.window.request_redraw();
                inner.render(delta, elapsed);
                self.last = new;
            }
            _ => (),
        }
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