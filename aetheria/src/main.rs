#![feature(cstr_from_bytes_until_nul)]
#![feature(let_chains)]

use ash::{
    vk::{self, ShaderStageFlags},
    Entry,
};

pub mod vulkan;
use vulkan::*;

fn create_window() -> (winit::event_loop::EventLoop<()>, winit::window::Window) {
    let event_loop = winit::event_loop::EventLoop::new();
    let window = winit::window::WindowBuilder::new()
        .build(&event_loop)
        .unwrap();
    (event_loop, window)
}

struct VulkanContext {
    instance: Instance,
    surface: Surface,
    device: Device,
    swapchain: Swapchain,
    renderpass: Renderpass,
    pipeline: GraphicsPipeline,
    shaders: vulkan::graphics::Shaders,
    framebuffers: Vec<vk::Framebuffer>,
    command_pool: CommandPool,

    image_available: vk::Semaphore,
    render_finished: vk::Semaphore,
    in_flight: vk::Fence,
}

unsafe fn destroy_swapchain(ctx: &mut VulkanContext) {
    ctx.device.device_wait_idle().unwrap();

    ctx.framebuffers
        .iter()
        .for_each(|framebuffer| ctx.device.destroy_framebuffer(*framebuffer, None));
    ctx.device.destroy_pipeline(*ctx.pipeline, None);
    ctx.device
        .destroy_pipeline_layout(ctx.pipeline.layout, None);
    ctx.device.destroy_render_pass(*ctx.renderpass, None);
    ctx.swapchain
        .image_views
        .iter()
        .for_each(|view| ctx.device.destroy_image_view(*view, None));
    ctx.device
        .extensions
        .swapchain
        .as_ref()
        .unwrap()
        .destroy_swapchain(*ctx.swapchain, None);
}

fn recreate_swapchain(ctx: &mut VulkanContext, window: &winit::window::Window) {
    unsafe { destroy_swapchain(ctx) };

    ctx.swapchain = vulkan::Swapchain::new(&ctx.instance, &ctx.surface, &ctx.device, window)
        .expect("Swapchain recreation failed");
    ctx.renderpass = vulkan::Renderpass::new(&ctx.device, ctx.swapchain.format)
        .expect("Swapchain recreation failed");
    ctx.pipeline = vulkan::GraphicsPipeline::new(
        &ctx.device,
        &ctx.renderpass,
        ctx.shaders.clone(),
        ctx.swapchain.extent,
    )
    .expect("Swapchain recreation failed");
    ctx.framebuffers = std::iter::zip(
        ctx.swapchain.images.clone(),
        ctx.swapchain.image_views.clone(),
    )
    .map(|(image, view)| {
        ctx.renderpass
            .create_framebuffer(&ctx.device, &image, &view)
            .unwrap()
    })
    .collect();
}

fn render(ctx: &mut VulkanContext, window: &winit::window::Window) {
    let mut frame_rendered = false;
    while (!frame_rendered) {
        unsafe {
            ctx.device
                .wait_for_fences(&[ctx.in_flight], true, u64::MAX)
                .unwrap();

            let swapchain_khr = ctx.device.extensions.swapchain.as_ref().unwrap();

            let acquire_result = swapchain_khr.acquire_next_image(
                ctx.swapchain.swapchain,
                u64::MAX,
                ctx.image_available,
                vk::Fence::null(),
            );

            let image_index = match acquire_result {
                Ok((image_index, _)) => image_index,
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    recreate_swapchain(ctx, window);
                    continue;
                }
                Err(e) => panic!("{}", e),
            };

            ctx.device.reset_fences(&[ctx.in_flight]).unwrap();

            let cmd = ctx
                .command_pool
                .allocate(&ctx.device)
                .unwrap()
                .begin(&ctx.device)
                .unwrap()
                .begin_renderpass(
                    &ctx.device,
                    &ctx.renderpass,
                    &ctx.framebuffers[image_index as usize],
                    ctx.swapchain.extent,
                )
                .bind_pipeline(&ctx.device, &ctx.pipeline)
                .draw(
                    &ctx.device,
                    vulkan::command::DrawOptions {
                        vertex_count: 3,
                        instance_count: 1,
                        ..Default::default()
                    },
                )
                .end_renderpass(&ctx.device)
                .end(&ctx.device)
                .unwrap();

            let wait_semaphores = &[ctx.image_available];
            let signal_semaphores = &[ctx.render_finished];
            let command_buffers = &[*cmd];
            let submit_info = vk::SubmitInfo::builder()
                .wait_semaphores(wait_semaphores)
                .wait_dst_stage_mask(&[vk::PipelineStageFlags::COLOR_ATTACHMENT_OUTPUT])
                .command_buffers(command_buffers)
                .signal_semaphores(signal_semaphores);

            ctx.device
                .queue_submit(
                    ctx.device.queues.graphics.queue,
                    &[*submit_info],
                    ctx.in_flight,
                )
                .unwrap();

            let swapchains = &[ctx.swapchain.swapchain];
            let image_indices = &[image_index];
            let present_info = vk::PresentInfoKHR::builder()
                .wait_semaphores(signal_semaphores)
                .swapchains(swapchains)
                .image_indices(image_indices);

            let present_result =
                swapchain_khr.queue_present(ctx.device.queues.present.queue, &present_info);

            match present_result {
                Err(vk::Result::ERROR_OUT_OF_DATE_KHR) => {
                    recreate_swapchain(ctx, window);
                }
                Err(e) => panic!("{}", e),
                _ => (),
            };

            frame_rendered = true;
        }
    }
}

fn main() {
    tracing_subscriber::fmt::init();

    let (event_loop, window) = create_window();

    let entry = Entry::linked();
    let instance = vulkan::Instance::new(&entry).expect("Vulkan instance creation failed");
    let surface = vulkan::Surface::new(&instance, &window).expect("Vulkan surface creation failed");
    let device =
        unsafe { vulkan::Device::new(&instance, &surface).expect("Vulkan device creation failed") };
    let mut swapchain = vulkan::Swapchain::new(&instance, &surface, &device, &window)
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
    let mut renderpass = vulkan::Renderpass::new(&device, swapchain.format)
        .expect("Vulkan renderpass creation failed");
    let shaders = vulkan::graphics::Shaders {
        vertex: Some(vertex_shader),
        fragment: Some(fragment_shader),
    };
    let mut pipeline =
        vulkan::GraphicsPipeline::new(&device, &renderpass, shaders.clone(), swapchain.extent)
            .expect("Vulkan pipleine creation failed");

    let mut framebuffers: Vec<ash::vk::Framebuffer> =
        std::iter::zip(swapchain.images.clone(), swapchain.image_views.clone())
            .map(|(image, view)| {
                renderpass
                    .create_framebuffer(&device, &image, &view)
                    .unwrap()
            })
            .collect();

    let command_pool = vulkan::CommandPool::new(&device).unwrap();

    let semaphore_info = vk::SemaphoreCreateInfo::builder();
    let fence_info = vk::FenceCreateInfo::builder().flags(vk::FenceCreateFlags::SIGNALED);

    let image_available = unsafe { device.create_semaphore(&semaphore_info, None).unwrap() };
    let render_finished = unsafe { device.create_semaphore(&semaphore_info, None).unwrap() };
    let in_flight = unsafe { device.create_fence(&fence_info, None).unwrap() };

    let mut context = VulkanContext {
        instance,
        surface,
        device,
        swapchain,
        renderpass,
        shaders,
        pipeline,
        framebuffers,
        command_pool,
        image_available,
        render_finished,
        in_flight,
    };

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
            winit::event::Event::MainEventsCleared => {
                render(&mut context, &window);
            }
            _ => {}
        }
    })
}
