use ash::vk;

use crate::Finish;

// Recorder state that is stored within the recorders that is dynamically bound to command buffers
#[derive(Default)]
pub(crate) struct State {
    pub(crate) commands: Vec<super::Command>,
    pub(crate) barriers: Vec<super::Barrier>,
}


impl Finish for State {
    unsafe fn finish(self, device: &ash::Device, buffer: vk::CommandBuffer) {
        for command in self.commands {
            command.finish(device, buffer);
        }
    }
}

// A recorder can keep a command buffer cached until we flush it
// This is used to reduce the number of submissions we have to make to the GPU
pub struct Recorder {
    // Index of the used command buffer
    pub(crate) index: usize,
    
    // Current saved state of the recorder
    pub(crate) state: State,

    // Raw command buffer
    pub(crate) raw: vk::CommandBuffer,
}

// This is a submission of a command recorder
// The underlying command buffer might've not been submitted yet
pub struct Submission {
    // Index of the command buffer
    pub(crate) index: usize,
}