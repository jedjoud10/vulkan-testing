use ash::vk;

use crate::Recorder;

// Enum that contains all the types of commands that can be applied to buffers
pub(super) enum BufferCommand {
    BindIndexBuffer {
        buffer: vk::Buffer,
        offset: vk::DeviceSize,
        index_type: vk::IndexType
    },
    BindVertexBuffer {
        first_binding: u32,
        binding_count: u32,
        buffers: Vec<vk::Buffer>,
        offsets: Vec<vk::DeviceSize>,
    },
    CopyBuffer {
        src: vk::Buffer,
        dst: vk::Buffer,
        regions: Vec<vk::BufferCopy>,
    },
    CopyImageToBuffer {
        dst: vk::Buffer,
        src: vk::Image,
        layout: vk::ImageLayout,
        regions: Vec<vk::BufferImageCopy>,
    },
    FillBuffer {
        src: vk::Buffer,
        offset: vk::DeviceSize,
        size: vk::DeviceSize, 
        data: u32,
    },
    UpdateBuffer {
        src: vk::Buffer,
        offset: vk::DeviceSize,
        size: vk::DeviceSize,
        data: Vec<u8>,
    },
}

impl Recorder {
    // Bind an index buffer to the command buffer render pass
    pub unsafe fn bind_index_buffer(&mut self, buffer: vk::Buffer, offset: vk::DeviceSize, index_type: vk::IndexType) {
        self.state.commands.push(super::Command::Buffer(BufferCommand::BindIndexBuffer { buffer, offset, index_type }));
    }
    
    // Bind vertex buffers to the command buffer render pass
    pub unsafe fn bind_vertex_buffers(&mut self, first_binding: u32, binding_count: u32, buffers: Vec<vk::Buffer>, offsets: Vec<vk::DeviceSize>) {
        self.state.commands.push(super::Command::Buffer(BufferCommand::BindVertexBuffer { first_binding, binding_count, buffers, offsets }));
    }
    
    // Copy a buffer to another buffer in GPU memory
    pub unsafe fn copy_buffer(&mut self, src: vk::Buffer, dst: vk::Buffer, regions: Vec<vk::BufferCopy>) {

    }
    
    // Copy an image to a buffer in GPU memory
    pub unsafe fn copy_image_to_buffer(&mut self, buffer: vk::Buffer, image: vk::Image, layout: vk::ImageLayout, regions: Vec<vk::BufferImageCopy>) {

    }
    
    // Fill a buffer with a specific value (either 1 or 0)
    pub unsafe fn cmd_fill_buffer(&mut self, buffer: vk::Buffer, offset: vk::DeviceSize, size: vk::DeviceSize, data: u32) {

    }

    // Update the buffer using memory that is directly stored within the command buffer
    pub unsafe fn cmd_update_buffer(&mut self, buffer: vk::Buffer, offset: vk::DeviceSize, size: vk::DeviceSize, data: Vec<u8>) {

    }
}