mod app;
mod render;

#[allow(dead_code, unused_variables, unused_imports)]
use anyhow::{anyhow, Result};

// winit related imports (window abstraction)
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::WindowBuilder;

fn main() -> anyhow::Result<()> {
    dotenv::dotenv().ok();
    pretty_env_logger::init();

    // Create window
    let event_loop = EventLoop::new();
    let window = WindowBuilder::new()
        .with_title("Vulkanalia Tutorial")
        // using the logical size will be dpi-scaled
        .with_inner_size(LogicalSize::new(1024, 768))
        .build(&event_loop)?;

    let mut app = unsafe { app::App::create(&window)? };
    let mut destroying = false;
    event_loop.run(move |event, _, control_flow| {
        // poll for events, even if none is available
        *control_flow = ControlFlow::Poll;

        match event {
            // render a new frame, if all events other than the RequestRequested have
            // been cleared
            Event::MainEventsCleared if !destroying => unsafe { app.render(&window) }.unwrap(),
            // emitted, if the OS sends an event to the winit window (specifically
            // a request to close the window)
            Event::WindowEvent {
                event: WindowEvent::CloseRequested,
                ..
            } => {
                destroying = true;
                *control_flow = ControlFlow::Exit;
                log::debug!("Exit...");
                unsafe { app.destroy(); }
            }
            _ => {}
        }
    });
}
