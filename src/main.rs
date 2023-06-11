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
        renderer.render();
    });
    unsafe { renderer.vulkan_context.device.device_wait_idle().unwrap() };
    let mut renderer = renderer;
    renderer.destroy();
}
