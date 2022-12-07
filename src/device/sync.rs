use std::ffi::CString;

use crate::{Adapter, Instance, Queue, Device};
use ash::vk::{self, DeviceCreateInfo, DeviceQueueCreateInfo};

use gpu_allocator::{
    vulkan::{
        Allocation, AllocationCreateDesc, Allocator,
        AllocatorCreateDesc,
    },
    MemoryLocation,
};
use parking_lot::Mutex;

impl Device {
    // Create a single simple semaphore
    pub unsafe fn create_semaphore(&self) -> vk::Semaphore {
        self.device
            .create_semaphore(&Default::default(), None)
            .unwrap()
    }

    // Create a single simple fence
    pub unsafe fn create_fence(&self) -> vk::Fence {
        self.device.create_fence(&Default::default(), None).unwrap()
    }
}