use std::ffi::CString;

use crate::{Adapter, Instance};
use ash::vk::{self, DeviceCreateInfo, DeviceQueueCreateInfo};

use gpu_allocator::{
    vulkan::{
        Allocation, AllocationCreateDesc, Allocator,
        AllocatorCreateDesc,
    },
    MemoryLocation,
};
use parking_lot::Mutex;

// This is a logical device that can run multiple commands and that can create Vulkan objects
pub struct Device {
    pub device: ash::Device,
    pub(crate) allocator: Mutex<Allocator>,
}

impl Device {
    // Create a new logical device from the physical adapter
    pub unsafe fn new(
        instance: &Instance,
        adapter: &Adapter,
    ) -> Device {
        // Get the graphics and present queue family
        let family = crate::Queue::pick_queue_family(
            &adapter.queue_family_properties,
            adapter,
            true,
            vk::QueueFlags::GRAPHICS,
        );

        // Create the queue create infos
        let create_infos = (std::iter::once(family))
            .map(|family| {
                *DeviceQueueCreateInfo::builder()
                    .queue_priorities(&[1.0])
                    .queue_family_index(family as u32)
            })
            .collect::<Vec<_>>();

        // Create logical device create info
        let required_device_extensions =
            crate::global::required_device_extensions();
        let logical_device_extensions = required_device_extensions
            .iter()
            .map(|s| s.as_ptr())
            .collect::<Vec<_>>();
        let logical_device_create_info = DeviceCreateInfo::builder()
            .queue_create_infos(&create_infos)
            .enabled_extension_names(&logical_device_extensions)
            .enabled_features(&adapter.features);

        // Create the logical device
        let device = instance
            .instance
            .create_device(
                adapter.raw,
                &logical_device_create_info,
                None,
            )
            .expect("Could not create the logical device");
        log::debug!("Created the Vulkan device successfully");

        // Pick allocator debug settings
        #[cfg(debug_assertions)]
        let debug_settings = gpu_allocator::AllocatorDebugSettings {
            log_memory_information: false,
            log_leaks_on_shutdown: true,
            store_stack_traces: false,
            log_allocations: true,
            log_frees: false,
            log_stack_traces: false,
        };

        // No debugging
        #[cfg(not(debug_assertions))]
        let debug_settings =
            gpu_allocator::AllocatorDebugSettings::default();

        // Create a memory allocator (gpu_allocator)
        let allocator = Allocator::new(&AllocatorCreateDesc {
            instance: instance.instance.clone(),
            device: device.clone(),
            physical_device: adapter.raw,
            debug_settings,
            buffer_device_address: false,
        })
        .unwrap();
        log::debug!(
            "Created the Vulkan memory allocator using gpu-allocator"
        );

        // Drop the cstrings
        drop(required_device_extensions);

        // Le logical device
        let device = Device {
            device,
            allocator: Mutex::new(allocator),
        };

        device
    }

    // Destroy the logical device
    pub unsafe fn destroy(self) {
        self.device.device_wait_idle().unwrap();
        self.device.destroy_device(None);
    }
}
