use softbuffer::Context;
use softbuffer::Surface;
use std::num::NonZeroU32;
use std::rc::Rc;
use std::time::Duration;
use std::time::Instant;
use winit::application::ApplicationHandler;
use winit::event::ElementState;
use winit::event::MouseButton;
use winit::event::WindowEvent;
use winit::event_loop::ActiveEventLoop;
use winit::event_loop::EventLoop;
use winit::keyboard::KeyCode;
use winit::keyboard::PhysicalKey;
use winit::window::CursorIcon;
use winit::window::Window;
use winit::window::WindowId;

#[derive(Clone, Copy, Debug)]
pub enum Color {
    Red = 0xef4444,
    Green = 0x22c55e,
    Blue = 0x3b82f6,
    White = 0xffffff,
}

struct MyApplication {
    window: Option<Rc<Window>>,
    window_size: (u32, u32),
    canvas: Vec<u32>,
    context: Option<Context<Rc<Window>>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,

    active_color: Color,

    is_left_mouse_down: bool,
    is_right_mouse_down: bool,

    last_mouse_position: (f64, f64),
    mouse_position: (f64, f64),

    undo_stack: Vec<Vec<u32>>,
    redo_stack: Vec<Vec<u32>>,

    is_control_key_pressed: bool,

    last_instant: Instant,
    frame_count: i32,
    fps: i32,
}

impl Default for MyApplication {
    fn default() -> Self {
        Self {
            window: None,
            context: None,
            surface: None,
            canvas: Vec::new(),
            window_size: (800, 600),
            active_color: Color::White,
            is_left_mouse_down: false,
            is_right_mouse_down: false,
            mouse_position: (0.0, 0.0),
            last_mouse_position: (0.0, 0.0),
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),

            is_control_key_pressed: false,

            last_instant: Instant::now(),
            frame_count: 0,
            fps: 0,
        }
    }
}

impl ApplicationHandler for MyApplication {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window_attributes = Window::default_attributes()
            .with_cursor(CursorIcon::Pointer)
            .with_title("Draw On Screen");

        let window = Rc::new(event_loop.create_window(window_attributes).unwrap());

        let context = Context::new(window.clone()).unwrap();
        let surface = Surface::new(&context, window.clone()).unwrap();

        let size = window.inner_size();
        self.window_size = (size.width, size.height);

        // Initialize canvas with white background (0xFFFFFFFF = white in ARGB)
        self.canvas = vec![0x00000000; (size.width * size.height) as usize];

        self.window = Some(window);
        self.context = Some(context);
        self.surface = Some(surface);
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        _window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::CloseRequested => {
                event_loop.exit();
            }
            WindowEvent::ModifiersChanged(modifiers) => {
                self.is_control_key_pressed = modifiers.state().control_key()
            }
            WindowEvent::RedrawRequested => {
                self.frame_count += 1;

                let now = Instant::now();
                let duration = now.duration_since(self.last_instant);

                if duration >= Duration::from_secs(1) {
                    self.fps = self.frame_count;
                    self.frame_count = 0;
                    self.last_instant = now;

                    println!("FPS: {}", self.fps);
                }

                self.render()
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    // If button 1 was pressed on keyhboard
                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit1) {
                        self.active_color = Color::Red;
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit2) {
                        self.active_color = Color::Green;
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit3) {
                        self.active_color = Color::Blue;
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::Digit4) {
                        self.active_color = Color::White;
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::Backspace) {
                        self.clear_canvas();
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::KeyZ)
                        && self.is_control_key_pressed
                    {
                        if let Some(prev_canvas) = self.undo_stack.pop() {
                            self.redo_stack.push(self.canvas.clone());
                            self.canvas = prev_canvas;
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }

                    if event.physical_key == PhysicalKey::Code(KeyCode::KeyR)
                        && self.is_control_key_pressed
                    {
                        if let Some(next_canvas) = self.redo_stack.pop() {
                            self.undo_stack.push(self.canvas.clone());
                            self.canvas = next_canvas;
                            if let Some(window) = &self.window {
                                window.request_redraw();
                            }
                        }
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.mouse_position = (position.x, position.y);

                if self.is_left_mouse_down {
                    self.draw_line(
                        self.last_mouse_position,
                        self.mouse_position,
                        self.active_color as u32,
                        // 1,
                    );
                }

                self.last_mouse_position = self.mouse_position;

                if self.is_right_mouse_down {
                    self.clear_circle(self.mouse_position, 20.0); // 10.0 is radius
                }

                if self.is_right_mouse_down || self.is_left_mouse_down {
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }

            WindowEvent::MouseInput { button, state, .. } => {
                if button == MouseButton::Left && state == ElementState::Pressed {
                    self.undo_stack.push(self.canvas.clone());
                    self.redo_stack.clear();
                    self.is_left_mouse_down = true;
                } else {
                    self.is_left_mouse_down = false;
                }

                if button == MouseButton::Right && state == ElementState::Pressed {
                    self.undo_stack.push(self.canvas.clone()); // Save before erasing
                    self.redo_stack.clear();
                    self.is_right_mouse_down = true;
                } else {
                    self.is_right_mouse_down = false;
                }

                if self.is_right_mouse_down {
                    self.clear_circle(self.mouse_position, 20.0); // 10.0 is radius
                }

                self.render();
            }
            WindowEvent::Resized(new_size) => {
                if new_size.width > 0 && new_size.height > 0 {
                    let old_width = self.window_size.0;
                    let old_height = self.window_size.1;
                    let new_width = new_size.width;
                    let new_height = new_size.height;

                    let mut new_canvas = vec![0x00000000; (new_width * new_height) as usize];

                    let copy_width = old_width.min(new_width);
                    let copy_height = old_height.min(new_height);

                    for y in 0..copy_height {
                        for x in 0..copy_width {
                            let old_idx = (y * old_width + x) as usize;
                            let new_idx = (y * new_width + x) as usize;

                            if old_idx < self.canvas.len() && new_idx < new_canvas.len() {
                                new_canvas[new_idx] = self.canvas[old_idx];
                            }
                        }
                    }

                    self.canvas = new_canvas;
                    self.window_size = (new_width, new_height);

                    if let Some(surface) = &mut self.surface {
                        surface
                            .resize(
                                NonZeroU32::new(new_width).unwrap(),
                                NonZeroU32::new(new_height).unwrap(),
                            )
                            .unwrap();
                    }

                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            _ => (),
        }
    }
}

impl MyApplication {
    fn render(&mut self) {
        if let Some(surface) = &mut self.surface {
            let mut buffer = surface.buffer_mut().unwrap();

            buffer.copy_from_slice(&self.canvas);
            buffer.present().unwrap();
        }
    }

    fn draw_pixel(&mut self, x: u32, y: u32, color: u32) {
        if x < self.window_size.0 && y < self.window_size.1 {
            let index = (y * self.window_size.0 + x) as usize;

            if index < self.canvas.len() {
                self.canvas[index] = color;
            }
        }
    }

    fn draw_line(&mut self, start: (f64, f64), end: (f64, f64), color: u32) {
        let mut x0 = start.0 as i32;
        let mut y0 = start.1 as i32;
        let x1 = end.0 as i32;
        let y1 = end.1 as i32;

        let dx = (x1 - x0).abs();
        let dy = (y1 - y0).abs();

        let step_x = if x0 < x1 { 1 } else { -1 };
        let step_y = if y0 < y1 { 1 } else { -1 };

        let mut error = dx - dy;

        loop {
            // Draw the current pixel
            if x0 >= 0
                && y0 >= 0
                && x0 < self.window_size.0 as i32
                && y0 < self.window_size.1 as i32
            {
                self.draw_pixel(x0 as u32, y0 as u32, color);
            }

            // Check if we've reached the end point
            if x0 == x1 && y0 == y1 {
                break;
            }

            let error2 = 2 * error;

            if error2 > -dy {
                error -= dy;
                x0 += step_x;
            }

            if error2 < dx {
                error += dx;
                y0 += step_y;
            }
        }
    }

    fn clear_canvas(&mut self) {
        // Save current state to undo stack
        self.undo_stack.push(self.canvas.clone());
        self.redo_stack.clear(); // Clear redo history

        // Fill canvas with black transparent background
        self.canvas.fill(0x00000000);

        // Request a redraw
        if let Some(window) = &self.window {
            window.request_redraw();
        }
    }

    fn clear_circle(&mut self, center: (f64, f64), radius: f64) {
        let (cx, cy) = center;

        let x0 = (cx - radius).max(0.0) as u32;
        let x1 = (cx + radius).min(self.window_size.0 as f64) as u32;
        let y0 = (cy - radius).max(0.0) as u32;
        let y1 = (cy + radius).min(self.window_size.1 as f64) as u32;

        for y in y0..=y1 {
            for x in x0..=x1 {
                let dx = x as f64 - cx;
                let dy = y as f64 - cy;
                if dx * dx + dy * dy <= radius * radius {
                    self.draw_pixel(x, y, 0x00000000); // Transparent pixel
                }
            }
        }
    }
}

fn main() -> Result<(), impl std::error::Error> {
    let event_loop = EventLoop::new().unwrap();

    // Force x11 backend
    unsafe {
        std::env::set_var("WINIT_UNIX_BACKEND", "x11");
    }

    let mut my_applicaiton = MyApplication::default();

    event_loop.run_app(&mut my_applicaiton)
}
