#![feature(cstr_from_bytes_until_nul)]
#![feature(let_chains)]

use ash::Entry;

pub mod vulkan;

fn create_window() -> (winit::event_loop::EventLoop<()>, winit::window::Window) {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .unwrap();
    (event_loop, window)
}

fn main() {
    let (event_loop, window) = create_window();

    let entry = Entry::linked();
    let instance = vulkan::Instance::new(&entry).expect("Vulkan instance creation failed");
    let physicals = instance
        .get_physical_devices()
        .expect("Fetching physical devices failed");
    let physical = physicals.first().expect("No vulkan compatible devices");
    println!("{}", physical.properties.device_name);

    event_loop.run(move |event, _, control_flow| {
        control_flow.set_poll();

        match event {
            winit::event::Event::WindowEvent {
                event: winit::event::WindowEvent::CloseRequested,
                ..
            } => {
                control_flow.set_exit();
            }
            winit::event::Event::DeviceEvent { event: winit::event::DeviceEvent::Key(input), .. } => {
                if let Some(key) = input.virtual_keycode && key == winit::event::VirtualKeyCode::Escape {
                    control_flow.set_exit();
                }
            }
            _ => {}
        }
    })
}
