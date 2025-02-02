use std::ffi::{CStr, CString};
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

        let required_validation_layers = {
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
        let queue_family_index = queue::find_appropriate_queue_family_index(physical_device, queue_family_properties, &surface_loader, surface_khr);

        let queue_create_info = vk::DeviceQueueCreateInfo::default()
            .queue_priorities(&[1.0])
            .queue_family_index(queue_family_index as u32);
        let queue_create_infos = [queue_create_info];

        let device_features = vk::PhysicalDeviceFeatures::default();

        let device_extension_names = vec![
            ash::khr::swapchain::NAME,
        ];

        let device_extension_names_ptrs = device_extension_names .iter()
            .map(|cstr| cstr.as_ptr())
            .collect::<Vec<_>>();

        let device_create_info = vk::DeviceCreateInfo::default()
            .enabled_extension_names(&device_extension_names_ptrs)
            .enabled_features(&device_features)
            .queue_create_infos(&queue_create_infos);

        let device = instance.create_device(physical_device, &device_create_info, None).unwrap();
        log::info!("created device");   

        Self {
            window,
            instance,
            entry,
            device,
            physical_device,
            surface_loader,
            surface_khr,
            debug_messenger,
        }
    }

    pub unsafe fn destroy(self) {
        if let Some((inst, debug_messenger)) = self.debug_messenger {
            inst.destroy_debug_utils_messenger(debug_messenger, None);
            log::info!("destroyed debug utils messenger");
        }

        self.surface_loader.destroy_surface(self.surface_khr, None);
        log::info!("destroyed surface");

        self.device.destroy_device(None);
        log::info!("destroyed device");

        self.instance.destroy_instance(None);
        log::info!("destroyed instance");
    }
}

struct App(Option<InternalApp>);

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        unsafe {
            self.0 = Some(InternalApp::new(event_loop));
        }
    }

    fn window_event(&mut self, event_loop: &ActiveEventLoop, id: WindowId, event: WindowEvent) {
        match event {
            WindowEvent::CloseRequested => unsafe {
                event_loop.exit();
                self.0.take().unwrap().destroy();
            },
            WindowEvent::RedrawRequested => {
                
            }
            _ => (),
        }
    }
}

pub fn main() {
    env_logger::Builder::from_default_env().filter_level(log::LevelFilter::Debug).init();
    let event_loop = EventLoop::new().unwrap();
    let mut app = App(None);
    event_loop.run_app(&mut app).unwrap();
}