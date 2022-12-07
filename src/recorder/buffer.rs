use ash::vk;

use crate::Recorder;

// Enum that contains all the types of commands that can be applied to buffers
pub(crate) enum BufferCommand {
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
        self.state.commands.push(super::Command::Buffer(BufferCommand::CopyBuffer { src, dst, regions }));
    }
    
    // Copy an image to a buffer in GPU memory
    pub unsafe fn copy_image_to_buffer(&mut self, buffer: vk::Buffer, image: vk::Image, layout: vk::ImageLayout, regions: Vec<vk::BufferImageCopy>) {
        self.state.commands.push(super::Command::Buffer(BufferCommand::CopyImageToBuffer { dst: buffer, src: image, layout, regions }));
    }
    
    // Clear a buffer to zero
    pub unsafe fn cmd_clear_buffer(&mut self, buffer: vk::Buffer, offset: vk::DeviceSize, size: vk::DeviceSize) {
        self.state.commands.push(super::Command::Buffer(BufferCommand::FillBuffer { src: buffer, offset, size, data: 0 }));
    }

    // Update the buffer using memory that is directly stored within the command buffer
    pub unsafe fn cmd_update_buffer(&mut self, buffer: vk::Buffer, offset: vk::DeviceSize, size: vk::DeviceSize, data: Vec<u8>) {
        self.state.commands.push(super::Command::Buffer(BufferCommand::UpdateBuffer { src: buffer, offset, size, data }));
    }
}

impl super::Finish for BufferCommand {
    unsafe fn finish(self, device: &ash::Device, cmd: vk::CommandBuffer) {
        match self {
            BufferCommand::BindIndexBuffer { buffer, offset, index_type } => 
                device.cmd_bind_index_buffer(cmd, buffer, offset, index_type),
            BufferCommand::BindVertexBuffer { first_binding, binding_count, buffers, offsets } => 
                device.cmd_bind_vertex_buffers(cmd, first_binding, &buffers, &offsets),
            BufferCommand::CopyBuffer { src, dst, regions } => 
                device.cmd_copy_buffer(cmd, src, dst, &regions),
            BufferCommand::CopyImageToBuffer { dst, src, layout, regions } => 
                device.cmd_copy_image_to_buffer(cmd, src, layout, dst, &regions),
            BufferCommand::FillBuffer { src, offset, size, data } => 
                device.cmd_fill_buffer(cmd, src, offset, size, data),
            BufferCommand::UpdateBuffer { src, offset, size, data } => 
                device.cmd_update_buffer(cmd, src, offset, &data),
        }
    }
}