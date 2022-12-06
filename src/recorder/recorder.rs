use ash::vk;

// Recorder state that is stored within the recorders that is dynamically bound to command buffers
pub(crate) struct State {
    pub(super) commands: Vec<super::Command>,
    pub(super) barriers: Vec<super::Barrier>,
}

// A recorder can keep a command buffer cached until we flush it
// This is used to reduce the number of submissions we have to make to the GPU
pub struct Recorder {
    pub(super) state: State,
    pub(super) raw: vk::CommandBuffer,
}

// This is a submission of a command recorder
// The underlying command buffer might've not been submitted yet
pub struct Submission {
}