use std::{num::NonZeroU32, rc::Rc};

use softbuffer::{Context, Rect, Surface};
use winit::{
    application::ApplicationHandler,
    event::{self, ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes, WindowId},
};

#[derive(PartialEq, Eq)]
enum DrawState {
    Idle,
    Drawing,
}

// TODO: Understand Clone & Copy @ Roman
#[derive(Clone, Copy)]
enum ColorState {
    Red = 0x00ff0000,
    Green = 0x0000ff00,
    Blue = 0x000000ff,
    White = 0xffffffff,
    Black = 0xff000000,
}

struct DrawOnScreen {
    window: Option<Rc<Window>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,
    context: Option<Context<Rc<Window>>>,

    canvas: Vec<u32>,

    draw_state: DrawState,
    color_state: ColorState,

    last_position: Option<winit::dpi::PhysicalPosition<f64>>,
}

impl Default for DrawOnScreen {
    fn default() -> Self {
        Self {
            window: None,
            surface: None,
            context: None,
            canvas: Vec::new(),
            draw_state: DrawState::Idle,
            color_state: ColorState::White,
            last_position: None,
        }
    }
}

impl ApplicationHandler for DrawOnScreen {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = WindowAttributes::default()
            .with_transparent(true)
            .with_title("Draw On Screen");

        let window = event_loop.create_window(window_attributes);

        match window {
            Ok(window) => {
                let window_inner_size = window.inner_size();

                let window = Rc::new(window);
                self.window = Some(window.clone());

                let context = Context::new(window.clone());

                match context {
                    Ok(context) => {
                        let surface = Surface::new(&context, window.clone());

                        self.context = Some(context);

                        match surface {
                            Ok(surface) => {
                                // let softbuffer = surface.buffer_mut();
                                //
                                // match softbuffer {
                                //     Ok(mut softbuffer) => {
                                //         softbuffer.copy_from_slice(&self.canvas);
                                //         softbuffer.present().unwrap();
                                //     }
                                //     Err(err) => println!("Failed to create soft buffer: {}", err),
                                // }

                                self.surface = Some(surface);
                            }
                            Err(err) => println!("Failed to create surface: {}", err),
                        };
                    }
                    Err(err) => println!("Failed to create context: {}", err),
                };
            }
            Err(err) => println!("Failed to create window: {}", err),
        };
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: event::WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => event_loop.exit(),
            WindowEvent::RedrawRequested => {
                println!("[RedrawRequested]");

                match self.draw_state {
                    DrawState::Idle => println!("Idle"),
                    DrawState::Drawing => println!("Drawing"),
                }

                if let Some(surface) = &mut self.surface {
                    let softbuffer = surface.buffer_mut();

                    match softbuffer {
                        Ok(mut softbuffer) => {
                            softbuffer.copy_from_slice(&self.canvas);
                            softbuffer.present().unwrap();
                        }
                        Err(err) => println!("Failed to create soft buffer: {}", err),
                    }
                }
            }
            WindowEvent::KeyboardInput {
                device_id,
                event,
                is_synthetic,
            } => {
                if !event.repeat && event.state == ElementState::Pressed {
                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit1) {
                        self.color_state = ColorState::White;
                        println!("White");
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit2) {
                        self.color_state = ColorState::Red;
                        println!("Red");
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit3) {
                        self.color_state = ColorState::Green;
                        println!("Green");
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit4) {
                        self.color_state = ColorState::Blue;
                        println!("Blue");
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit5) {
                        self.color_state = ColorState::Black;
                        println!("Black");
                    }
                }
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                if let Some(window) = &self.window {
                    if self.draw_state == DrawState::Drawing {
                        let index = (position.y as usize * window.inner_size().width as usize)
                            + position.x as usize;

                        self.canvas[index] = self.color_state as u32;

                        println!(
                            "[CursorMoved]: Position is {}x{} (i - {})",
                            position.x, position.y, index
                        );

                        if let Some(surface) = &mut self.surface {
                            let window_size = window.inner_size();
                            let buffer_result = surface.buffer_mut();

                            match buffer_result {
                                Ok(mut softbuffer) => {
                                    let width = window_size.width as usize;
                                    let x = position.x as usize;
                                    let y = position.y as usize;

                                    let index = y * width + x;

                                    // Write pixel directly into softbuffer instead of using self.canvas
                                    softbuffer[index] = self.color_state as u32;

                                    // Now only present this one pixel
                                    softbuffer
                                        .present_with_damage(&[Rect {
                                            x: x as u32,
                                            y: y as u32,
                                            width: NonZeroU32::new(1).unwrap(),
                                            height: NonZeroU32::new(1).unwrap(),
                                        }])
                                        .unwrap();
                                }
                                Err(err) => println!("Failed to create soft buffer: {}", err),
                            }
                        }

                        // window.request_redraw();
                    }
                }
            }
            WindowEvent::MouseInput {
                device_id,
                state,
                button,
            } => {
                if let Some(window) = &self.window {
                    match button {
                        MouseButton::Left => {
                            if state == winit::event::ElementState::Pressed {
                                self.draw_state = DrawState::Drawing;
                                window.request_redraw();
                            }

                            if state == winit::event::ElementState::Released {
                                self.draw_state = DrawState::Idle;
                                self.last_position = None;
                                window.request_redraw();
                            }
                        }
                        _ => (),
                    }
                }
            }
            WindowEvent::Resized(size) => {
                println!("[Resized]: Size is {}x{}", size.width, size.height);
                if size.width > 0 && size.height > 0 {
                    self.canvas =
                        vec![ColorState::Black as u32; (size.width * size.height) as usize];

                    if let Some(surface) = &mut self.surface {
                        surface
                            .resize(
                                NonZeroU32::new(size.width).unwrap(),
                                NonZeroU32::new(size.height).unwrap(),
                            )
                            .unwrap()
                    }
                }
            }
            _ => (),
        }
    }
}

fn main() {
    let event_loop = EventLoop::new();

    match event_loop {
        Ok(event_loop) => {
            let mut draw_on_screen = DrawOnScreen::default();
            let app = event_loop.run_app(&mut draw_on_screen);

            match app {
                Ok(_) => println!("Event loop exited"),
                Err(err) => println!("Failed to run event loop: {}", err),
            };
        }
        Err(err) => println!("Failed to create event loop: {}", err),
    };
}
