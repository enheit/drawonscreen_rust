use std::{num::NonZeroU32, rc::Rc};

use softbuffer::{Context, Rect, Surface};
use winit::{
    application::ApplicationHandler,
    event::{self, ElementState, MouseButton, WindowEvent},
    event_loop::EventLoop,
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowAttributes},
};

const WIDTH: usize = 100;
const HEIGHT: usize = 100;

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
    fn resumed(&mut self, event_loop: &winit::event_loop::ActiveEventLoop) {
        let window_attributes = WindowAttributes::default().with_title("Draw On Screen");

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
        event_loop: &winit::event_loop::ActiveEventLoop,
        window_id: winit::window::WindowId,
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
                }
            }
            WindowEvent::CursorMoved {
                device_id,
                position,
            } => {
                if let Some(window) = &self.window {
                    if self.draw_state == DrawState::Drawing {
                        let window_size = window.inner_size();
                        let width = window_size.width as usize;
                        let height = window_size.height as usize;
                        let x = position.x as usize;
                        let y = position.y as usize;

                        if let Some(prev_pos) = self.last_position {
                            let px = prev_pos.x as usize;
                            let py = prev_pos.y as usize;

                            // Call this before any mutable borrows
                            let line = self.draw_line((px, py), (x, y), width, height);

                            // Draw to canvas
                            for (lx, ly) in &line {
                                let index = ly * width + lx;
                                if index < self.canvas.len() {
                                    self.canvas[index] = self.color_state as u32;
                                }
                            }

                            // Update surface
                            if let Some(surface) = &mut self.surface {
                                let buffer_result = surface.buffer_mut();
                                match buffer_result {
                                    Ok(mut softbuffer) => {
                                        for (lx, ly) in &line {
                                            let index = ly * width + lx;
                                            if index < softbuffer.len() {
                                                softbuffer[index] = self.color_state as u32;
                                            }
                                        }

                                        // Bounding box for present_with_damage
                                        let min_x =
                                            line.iter().map(|(x, _)| *x).min().unwrap_or(x) as u32;
                                        let max_x =
                                            line.iter().map(|(x, _)| *x).max().unwrap_or(x) as u32;
                                        let min_y =
                                            line.iter().map(|(_, y)| *y).min().unwrap_or(y) as u32;
                                        let max_y =
                                            line.iter().map(|(_, y)| *y).max().unwrap_or(y) as u32;

                                        let rect = Rect {
                                            x: min_x,
                                            y: min_y,
                                            width: NonZeroU32::new((max_x - min_x + 1).max(1))
                                                .unwrap(),
                                            height: NonZeroU32::new((max_y - min_y + 1).max(1))
                                                .unwrap(),
                                        };

                                        softbuffer.present_with_damage(&[rect]).unwrap();
                                    }
                                    Err(err) => println!("Failed to create soft buffer: {}", err),
                                }
                            }
                        }

                        self.last_position = Some(position);
                    }
                }

                // if let Some(window) = &self.window {
                //     if self.draw_state == DrawState::Drawing {
                //         let index = (position.y as usize * window.inner_size().width as usize)
                //             + position.x as usize;
                //
                //         self.canvas[index] = self.color_state as u32;
                //
                //         println!(
                //             "[CursorMoved]: Position is {}x{} (i - {})",
                //             position.x, position.y, index
                //         );
                //
                //         if let Some(surface) = &mut self.surface {
                //             let window_size = window.inner_size();
                //             let buffer_result = surface.buffer_mut();
                //
                //             match buffer_result {
                //                 Ok(mut softbuffer) => {
                //                     // let width = window_size.width as usize;
                //                     // let x = position.x as usize;
                //                     // let y = position.y as usize;
                //                     //
                //                     // let index = y * width + x;
                //                     //
                //                     // // Write pixel directly into softbuffer instead of using self.canvas
                //                     // softbuffer[index] = self.color_state as u32;
                //                     //
                //                     // // Now only present this one pixel
                //                     // softbuffer
                //                     //     .present_with_damage(&[Rect {
                //                     //         x: x as u32,
                //                     //         y: y as u32,
                //                     //         width: NonZeroU32::new(1).unwrap(),
                //                     //         height: NonZeroU32::new(1).unwrap(),
                //                     //     }])
                //                     //     .unwrap();
                //                 }
                //                 Err(err) => println!("Failed to create soft buffer: {}", err),
                //             }
                //         }
                //
                //         // window.request_redraw();
                //     }
                // }
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
                    self.canvas = vec![0x00000000; (size.width * size.height) as usize];

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

impl DrawOnScreen {
    fn draw_line(
        &mut self,
        start: (usize, usize),
        end: (usize, usize),
        width: usize,
        height: usize,
    ) -> Vec<(usize, usize)> {
        let mut points = vec![];

        let (x0, y0) = start;
        let (x1, y1) = end;

        let dx = (x1 as isize - x0 as isize).abs();
        let dy = -(y1 as isize - y0 as isize).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        let mut x = x0 as isize;
        let mut y = y0 as isize;

        loop {
            if x >= 0 && y >= 0 && x < width as isize && y < height as isize {
                points.push((x as usize, y as usize));
            }

            if x == x1 as isize && y == y1 as isize {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x += sx;
            }
            if e2 <= dx {
                err += dx;
                y += sy;
            }
        }

        points
    }
}
