use std::cell::RefCell;

use winit::{
    event::{ElementState, Event, KeyboardInput, VirtualKeyCode, WindowEvent},
    event_loop::{ControlFlow, EventLoop},
    platform::run_return::EventLoopExtRunReturn, window::WindowBuilder,
};


pub struct WindowContext {
    pub window: winit::window::Window,
    pub event_loop: RefCell<EventLoop<()>>,
}

impl WindowContext {
    pub fn new(width: u32, height: u32) -> Self {
        let event_loop = EventLoop::new();
        let window = WindowBuilder::new()
            .with_title("rend-vk")
            .with_inner_size(winit::dpi::LogicalSize::new(
                f64::from(width),
                f64::from(height),
            ))
            .build(&event_loop)
            .unwrap();
        WindowContext {
            window,
            event_loop: RefCell::new(event_loop)
        }
    }
    pub fn event_loop<F: FnMut()>(&self, mut on_event: F) {
        self.event_loop
            .borrow_mut()
            .run_return(|event, _, control_flow| {
                *control_flow = ControlFlow::Poll;
                match event {
                    Event::WindowEvent {
                        event:
                            WindowEvent::CloseRequested
                            | WindowEvent::KeyboardInput {
                                input:
                                    KeyboardInput {
                                        state: ElementState::Pressed,
                                        virtual_keycode: Some(VirtualKeyCode::Escape),
                                        ..
                                    },
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit,
                    Event::MainEventsCleared => on_event(),
                    _ => (),
                }
            });
    }
}
