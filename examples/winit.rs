use ash::vk;
use gpu_allocator::MemoryLocation;
use vulkan_abstraction::*;
use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

fn main() {
    // Create a default winit window and event loop
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let app_name = "Winit Example";
    let engine_name = "Placeholder Engine";

    // Create the Vulkan entry and instance
    let instance =
        unsafe { Instance::new(&window, app_name, engine_name) };

    unsafe {
        // Setup the vulkan state
        let surface = Surface::new(&instance, &window);
        let adapter = Adapter::pick(&instance, &surface);
        let device = Device::new(&instance, &adapter);
        let queue = Queue::new(&instance, &device, &adapter);
        let swapchain = Swapchain::new(
            &adapter, &surface, &device, &instance, &window, false,
        );

        // Create two buffers
        let (buffer1, mut alloc1) = device.create_buffer(
            4,
            vk::BufferUsageFlags::TRANSFER_SRC
                | vk::BufferUsageFlags::TRANSFER_DST,
            MemoryLocation::CpuToGpu,
            &queue,
        );
        let ptr = alloc1.mapped_slice_mut().unwrap();
        ptr.copy_from_slice(&[1, 2, 3, 4]);
        log::debug!("{:?}", ptr);
        let (buffer2, alloc2) = device.create_buffer(
            4,
            vk::BufferUsageFlags::TRANSFER_DST
                | vk::BufferUsageFlags::TRANSFER_SRC,
            MemoryLocation::CpuToGpu,
            &queue,
        );

        // Aquire a new recorder
        let mut recorder = queue.acquire(&device, false);
        let copy1 = vk::BufferCopy::builder()
            .size(4)
            .dst_offset(0)
            .src_offset(0);
        let copy2 = vk::BufferCopy::builder()
            .size(2)
            .dst_offset(2)
            .src_offset(0);
        recorder.copy_buffer(buffer1, buffer2, vec![*copy1, *copy2]);

        recorder.copy_buffer(buffer2, buffer1, vec![*copy1, *copy2]);
        queue.submit(&device, recorder);
        let mut recorder = queue.acquire(&device, true);
        queue.submit(&device, recorder);

        let ptr = alloc2.mapped_slice().unwrap();
        log::debug!("{:?}", ptr);

        // Destroy the buffers
        device.destroy_buffer(buffer1, alloc1);
        device.destroy_buffer(buffer2, alloc2);

        // Being the event loop
        event_loop.run(move |event, _, control_flow| {
            *control_flow = ControlFlow::Wait;

            match event {
                Event::WindowEvent {
                    event: WindowEvent::CloseRequested,
                    window_id,
                } if window_id == window.id() => {
                    *control_flow = ControlFlow::Exit
                }
                Event::MainEventsCleared => {}
                _ => (),
            }
        });
    }
}
