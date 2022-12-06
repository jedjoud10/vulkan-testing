use winit::{
    event::{Event, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};
use vulkan_abstraction::{*};

fn main() {
    // Create a default winit window and event loop
    env_logger::init();
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let app_name = "Winit Example";
    let engine_name = "Placeholder Engine";

    // Create the Vulkan entry and instance
    let instance = unsafe { Instance::new(
        &window,
        app_name,
        engine_name,
    ) };

    // Create a surface that we shall render to
    let surface = unsafe { Surface::new(&instance, &window) };

    // Pick a physical device (adapter)
    let adapter = unsafe { Adapter::pick(&instance, &surface) };

    // Create a new device with those queues
    let device = unsafe { Device::new(&instance, &adapter) };

    // Create the queues that we will submit to
    //let queues = unsafe { Queues::new(&instance, &device, &adapter) };

    // Being the event loop
    event_loop.run(move |event, _, control_flow| {
        *control_flow = ControlFlow::Wait;

        match event {
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                window_id,
            } if window_id == window.id() => *control_flow = ControlFlow::Exit,
            Event::MainEventsCleared => {
            },
            _ => (),
        }
    });
}