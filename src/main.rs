use anyhow::Result;
use winit::dpi::LogicalSize;
use winit::event::{Event, WindowEvent};
use winit::event_loop::{ControlFlow, EventLoop};
use winit::window::{Window, WindowBuilder};


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

    let mut app = unsafe {App::create(&window)?};
    let mut destroying = false;
    event_loop.run(move |event, _, control_flow| {
        // poll for events, even if none is available
        *control_flow = ControlFlow::Poll;

        match event {
            // render a new frame, if all events other than the RequestRequested have
            // been cleared
            Event::MainEventsCleared if !destroying =>
                unsafe { app.render(&window) }.unwrap(),
            // emitted, if the OS sends an event to the winit window (specifically
            // a request to close the window)
            Event::WindowEvent { event: WindowEvent::CloseRequested, ..} => {
                destroying = true;
                *control_flow = ControlFlow::Exit;
                log::info!("Hello");
                unsafe { app.destroy(); }
            }
            _ => {}
        }
    });
}

#[derive(Clone, Debug)]
struct App {}

// TODO: expose own safe wrapper around vulkan calls, which asserts the calling
// of the correct invariants of the vulkan API functions
impl App {
    /// creates the app
    unsafe fn create(window: &Window) -> Result<Self> {
        Ok(Self{})
    }

    /// renders one frame
    unsafe fn render(&mut self, window: &Window) -> Result<()> {
        Ok(())
    }

    /// destroy the app
    unsafe fn destroy(&mut self) {}
}

#[derive(Clone, Debug, Default)]
struct AppData{}
