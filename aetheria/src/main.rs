#![feature(cstr_from_bytes_until_nul)]
#![feature(let_chains)]

use ash::{vk::ShaderStageFlags, Entry};

pub mod vulkan;

fn create_window() -> (winit::event_loop::EventLoop<()>, winit::window::Window) {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .unwrap();
    (event_loop, window)
}

fn main() {
    tracing_subscriber::fmt::init();

    let (event_loop, window) = create_window();

    let entry = Entry::linked();
    let instance = vulkan::Instance::new(&entry).expect("Vulkan instance creation failed");
    let surface = vulkan::Surface::new(&instance, &window).expect("Vulkan surface creation failed");
    let device =
        unsafe { vulkan::Device::new(&instance, &surface).expect("Vulkan device creation failed") };
    let swapchain = vulkan::Swapchain::new(&instance, &surface, &device, &window)
        .expect("Vulkan swapchain creation failed");

    let vertex_shader = vulkan::graphics::Shader::new(
        &device,
        include_bytes!("../../assets/shaders/compiled/vertex.spv").to_vec(),
        ShaderStageFlags::VERTEX,
    )
    .unwrap();
    let fragment_shader = vulkan::graphics::Shader::new(
        &device,
        include_bytes!("../../assets/shaders/compiled/fragment.spv").to_vec(),
        ShaderStageFlags::FRAGMENT,
    )
    .unwrap();
    let renderpass = vulkan::Renderpass::new(&device, swapchain.format)
        .expect("Vulkan renderpass creation failed");
    let shaders = vulkan::graphics::Shaders {
        vertex: Some(vertex_shader),
        fragment: Some(fragment_shader),
    };
    let pipeline = vulkan::GraphicsPipeline::new(&device, &renderpass, shaders, swapchain.extent)
        .expect("Vulkan pipleine creation failed");

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
