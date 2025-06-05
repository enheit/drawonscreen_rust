use std::error::Error;

use winit::{
    application::ApplicationHandler,
    event::WindowEvent,
    event_loop::{ActiveEventLoop, EventLoop},
    window::{Window, WindowId},
};

struct DrawOnScreen {
    window: Option<Window>,
}

impl Default for DrawOnScreen {
    fn default() -> Self {
        Self { window: None }
    }
}

impl ApplicationHandler for DrawOnScreen {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes().with_title("Draw On Screen");

        let result = event_loop.create_window(window_attributes);

        match result {
            Ok(window) => {
                println!("Window created");
                window.request_redraw();
                self.window = Some(window);
            }
            Err(error) => {
                eprintln!("Failed to create window: {}", error);
                event_loop.exit();
            }
        }
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                println!("Window closed");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {}
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {}
            WindowEvent::MouseWheel {
                device_id,
                delta,
                phase,
            } => {}
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {}
            WindowEvent::RedrawRequested => {}
            _ => {}
        }
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let event_loop = EventLoop::new()?;
    let mut draw_on_screen = DrawOnScreen::default();

    event_loop.run_app(&mut draw_on_screen)?;

    Ok(())
}
