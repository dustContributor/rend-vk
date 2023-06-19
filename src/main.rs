use ash::vk;

use rend_vk::window::WindowContext;
use rend_vk::*;

fn main() {
    log4rs::init_file("log4rs.yaml", Default::default()).unwrap();
    log_panics::init();

    let window_context = WindowContext::new(1280, 720);
    let instance_extensions =
        ash_window::enumerate_required_extensions(&window_context.window).unwrap();
    let mut renderer = renderer::make_renderer(instance_extensions, |entry, instance, surface| {
        let surface_maybe =
            unsafe { ash_window::create_surface(entry, instance, &window_context.window, None) };
        match surface_maybe {
            Err(err) => err,
            Ok(sur) => {
                unsafe { surface.write(sur) };
                vk::Result::SUCCESS
            }
        }
    });
    window_context.event_loop(|| {
        let test_task = render_task::RenderTask {
            mesh_buffer_id: 1,
            instance_count: 1,
            kind: render_task::TaskKind::MeshStatic,
            resources: render_task::resource_array()
        };
        let fullscreen_task = render_task::RenderTask {
            mesh_buffer_id: 1,
            instance_count: 1,
            kind: render_task::TaskKind::Fullscreen,
            resources: render_task::resource_array()
        };
        renderer.add_task_to_queue(test_task);
        renderer.add_task_to_queue(fullscreen_task);
        renderer.render();
    });
    unsafe { renderer.vulkan_context.device.device_wait_idle().unwrap() };
    let mut renderer = renderer;
    renderer.destroy();
}
