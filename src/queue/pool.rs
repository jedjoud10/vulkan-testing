use std::sync::Arc;
use ash::vk;
use parking_lot::Mutex;
use crate::{State, Recorder, Submission, Device, Adapter, Instance};


bitflags::bitflags! {
    pub struct CommandBufferTags: u32 {
        // Can we chain this command buffer? Aka can we use multiple times whilst it is recording?
        const CHAINABLE = 1;

        // Is the command buffer currently recording?
        const RECORDING = 2;

        // Is the command buffer awaiting for execution?
        const PENDING = 4;
    }
}

// Abstraction around a Vulkan command buffer
pub(crate) struct CommandBuffer {
    // Underlying command buffer
    pub(crate) raw: vk::CommandBuffer,

    // State of the command buffer
    pub(crate) state: Mutex<Option<State>>,

    // Tags that are applied to this command buffer
    pub(crate) tags: Mutex<CommandBufferTags>,
}

// Abstraction around a Vulkan command pool
pub(crate) struct Pool {
    // Underlying pool
    pub(crate) pool: vk::CommandPool,

    // All the buffers that we allocated
    pub(crate) buffers: Vec<CommandBuffer>,
}

impl Pool {
    // Create a new command pool and pre-allocate it
    pub(crate) unsafe fn new(device: &Device, qfi: u32) -> Self {
        // Create the raw Vulkan command pool
        let pool_create_info = vk::CommandPoolCreateInfo::builder()
            .flags(vk::CommandPoolCreateFlags::RESET_COMMAND_BUFFER)
            .queue_family_index(qfi);
        let command_pool = device.device.create_command_pool(&pool_create_info, None).unwrap();

        // Allocate some new command buffers
        let allocate_info = vk::CommandBufferAllocateInfo::builder()
            .command_buffer_count(32)
            .command_pool(command_pool)
            .level(vk::CommandBufferLevel::PRIMARY);
        let buffers = device.device
            .allocate_command_buffers(&allocate_info)
            .unwrap().into_iter().map(|raw| {
                CommandBuffer {
                    raw,
                    state: Mutex::new(Some(State::default())),
                    tags: Mutex::new(CommandBufferTags::CHAINABLE),
                }
            });
        

        Self {
            pool: command_pool,
            buffers: buffers.collect(),
        }
    }   

    // Get the index of a free command buffer from this pool
    pub(crate) unsafe fn free(&self) -> usize {
        self.buffers.iter().position(|cmd_buffer| {
            let CommandBuffer { tags, .. } = cmd_buffer;
            let chainable = tags.lock().contains(CommandBufferTags::CHAINABLE);
            let recording = tags.lock().contains(CommandBufferTags::RECORDING);
            let pending = tags.lock().contains(CommandBufferTags::PENDING);
            (chainable && recording) && !pending || (!recording && !pending)
        }).unwrap()
    }
}
