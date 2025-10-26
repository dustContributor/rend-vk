use std::cell::RefCell;

use winit::{
    event::{ElementState, Event, KeyEvent, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    keyboard::{Key, NamedKey},
    platform::run_on_demand::EventLoopExtRunOnDemand,
    window::WindowBuilder,
};
pub struct WindowContext {
    pub window: winit::window::Window,
    pub event_loop: RefCell<EventLoop<()>>,
}

impl WindowContext {
    pub fn new(width: u32, height: u32) -> Self {
        let event_loop = EventLoop::new().unwrap();
        let window = WindowBuilder::new()
            .with_title("rend-vk")
            .with_inner_size(winit::dpi::LogicalSize::new(
                f64::from(width),
                f64::from(height),
            ))
            .with_visible(true)
            .build(&event_loop)
            .unwrap();
        WindowContext {
            window,
            event_loop: RefCell::new(event_loop),
        }
    }

    pub fn event_loop<F: FnMut()>(&self, mut on_event: F) -> Result<(), impl std::error::Error> {
        self.event_loop.borrow_mut().run_on_demand(|event, elwp| {
            elwp.set_control_flow(ControlFlow::Poll);
            match event {
                Event::WindowEvent {
                    event:
                        WindowEvent::CloseRequested
                        | WindowEvent::KeyboardInput {
                            event:
                                KeyEvent {
                                    state: ElementState::Pressed,
                                    logical_key: Key::Named(NamedKey::Escape),
                                    ..
                                },
                            ..
                        },
                    ..
                } => {
                    elwp.exit();
                }
                Event::AboutToWait => on_event(),
                _ => (),
            }
        })
    }
}
