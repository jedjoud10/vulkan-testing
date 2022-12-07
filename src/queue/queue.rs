use std::sync::Arc;
use ash::vk;
use parking_lot::Mutex;
use crate::{State, Recorder, Submission, Device, Adapter, Instance, Pool, CommandBufferTags};

// This will be the main queue that we will access and submit data into
// For now I only support a single queue cause I am a bit dumb
pub struct Queue {
    // Queue family index
    pub(crate) qfi: u32, 

    // Queue family properties
    pub(crate) properties: vk::QueueFamilyProperties,

    // Command pools that we can use
    pub(crate) pools: Vec<Pool>,

    // Main queue that we submit command buffers to
    pub(crate) queue: vk::Queue,
}

impl Queue {
    // Create the queue families, queues, and default pools
    pub unsafe fn new(instance: &Instance, device: &Device, adapter: &Adapter) -> Self {
        // Get the present and graphics queue family
        let family = adapter.queue_family_properties
            .iter()
            .enumerate()
            .position(|(i, props)| {
                // Check if the queue family supports the flags
                let flags = props.queue_flags.contains(vk::QueueFlags::GRAPHICS);

                // If the queue we must fetch must support presenting, fetch the physical device properties
                let presenting = !adapter.queue_family_surface_supported[i] || adapter.queue_family_surface_supported[i];
                flags && presenting
            })
            .unwrap() as u32;

        // Get the queue from the device
        let queue = device.device.get_device_queue(family, 0);
        log::debug!("Created the default graphics-present queue successfully");

        Self {
            qfi: family,
            properties: adapter.queue_family_properties[family as usize],
            pools: vec![Pool::new(device, family)],
            queue,
        }
    }


    // Find a queue that supports the specific flags
    pub(crate) unsafe fn pick_queue_family(
        family_properties: &[vk::QueueFamilyProperties],
        adapter: &Adapter,
        supports_presenting: bool,
        flags: vk::QueueFlags,
    ) -> u32 {
        family_properties
            .iter()
            .enumerate()
            .position(|(i, props)| {
                // Check if the queue family supporsts the flags
                let flags = props.queue_flags.contains(flags);

                // If the queue we must fetch must support presenting, fetch the physical device properties
                let presenting = !supports_presenting || adapter.queue_family_surface_supported[i];
                flags && presenting
            })
            .unwrap() as u32
    }

    // Aquire a new free command recorder that we can use to record commands
    // This might return a command buffer that is already in the recording state* 
    pub unsafe fn aquire(
        &self,
        device: &Device,
        chainable: bool,
        force: bool,
    ) -> Recorder {
        // Get the current thread's command pool
            // Allocate new one if not
        let pool = &self.pools[0];

        // Get a free command buffer 
            // Allocate new one if not
        let index = pool.free();
        log::debug!("Found a free command buffer at index {}", index);
        let buf = &pool.buffers[index];
        buf.tags.lock().set(CommandBufferTags::RECORDING, true);
        buf.tags.lock().set(CommandBufferTags::CHAINABLE, chainable);
        let state = buf.state.lock().take().unwrap();
        log::debug!("Currently chained commands: {}", state.commands.len());
        log::debug!("Currently chained barriers: {}", state.barriers.len());

        // Create the recorder
        Recorder {
            index,
            state,
            raw: buf.raw,
        }
    }


    // Submit the command buffer (this doesn't actually submit it, it only steals it's state)
    // You can use the "force" parameter to force the submission of this command buffer
    pub unsafe fn submit(&self, device: &Device, recorder: Recorder) -> Submission {
        log::debug!("Submitting (locally storing) command recorder");
        log::debug!("Currently stored commands: {}", recorder.state.commands.len());
        log::debug!("Currentl stored barriers: {}", recorder.state.barriers.len());

        device.device.begin_command_buffer(recorder.raw, &vk::CommandBufferBeginInfo::default()).unwrap();
        let state = recorder.state;
        crate::Finish::finish(state, &device.device, recorder.raw);
        device.device.end_command_buffer(recorder.raw).unwrap();

        let bufs = [recorder.raw];
        let info = vk::SubmitInfo::builder()
            .command_buffers(&bufs);

        device.device.queue_submit(self.queue, &[*info], vk::Fence::null()).unwrap();
        device.device.queue_wait_idle(self.queue).unwrap();


        Submission {
            index: recorder.index
        }
    }

    // Destroy the queue and the command pools
    pub unsafe fn destroy(&self) {

    }
}