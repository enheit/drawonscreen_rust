use std::{error::Error, num::NonZeroU32, rc::Rc};

use softbuffer::{Context, Rect, Surface};
use winit::{
    application::ApplicationHandler,
    dpi::PhysicalSize,
    event::{ElementState, MouseButton, WindowEvent},
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{CursorIcon, Window, WindowId},
};

#[derive(Clone, Copy)]
enum Color {
    Red = 0x00ef4444,
    Green = 0x0022c55e,
    Blue = 0x003b82f6,
    White = 0x00fafafa,
    Black = 0x000a0a0a,
}

#[derive(PartialEq, Eq)]
enum DrawState {
    Idle,
    Drawing,
    Erasing,
}

struct DrawOnScreen {
    window: Option<Rc<Window>>,
    context: Option<Context<Rc<Window>>>,
    surface: Option<Surface<Rc<Window>, Rc<Window>>>,

    pixels: Vec<u32>, // This will hold the current state of your drawing
    inner_size: PhysicalSize<u32>,
    position: Option<(i32, i32)>,
    last_position: Option<(i32, i32)>,

    pointer_color: Color,
    draw_state: DrawState,
    draw_brush_radius: i32,
    erase_brush_radius: i32,

    is_control_key_pressed: bool,

    undo_stack: Vec<Vec<u32>>,
    redo_stack: Vec<Vec<u32>>,
}

impl DrawOnScreen {
    fn save_state(&mut self) {
        // Save the current `self.pixels` state to undo stack
        self.undo_stack.push(self.pixels.clone());
        self.redo_stack.clear(); // Clear redo after new action
    }

    fn restore_state(&mut self, pixels: Vec<u32>) {
        if self.pixels.len() == pixels.len() {
            self.pixels.copy_from_slice(&pixels);
            // Request redraw after restoring state
            if let Some(window) = &self.window {
                window.request_redraw();
            }
        }
    }

    fn xy_to_index(&self, x: u32, y: u32, width: u32) -> usize {
        (y * width + x) as usize
    }

    // Optimized Bresenham line algorithm - returns only the points, no allocation in hot path
    fn bresenham_line_fast(
        &self,
        x0: i32,
        y0: i32,
        x1: i32,
        y1: i32,
        points: &mut Vec<(i32, i32)>,
    ) {
        // This function is currently empty in your provided code.
        // The implementation for Bresenham is directly in draw_interpolated_line.
        // If you intend to use bresenham_line_fast as a separate utility,
        // its implementation should be moved here. For now, it's not strictly
        // needed to fix the click drawing issue.
    }

    // Blend two colors with alpha blending
    fn blend_colors(&self, background: u32, foreground: u32, alpha: f32) -> u32 {
        let alpha = alpha.clamp(0.0, 1.0);
        let inv_alpha = 1.0 - alpha;

        let bg_r = ((background >> 16) & 0xFF) as f32;
        let bg_g = ((background >> 8) & 0xFF) as f32;
        let bg_b = (background & 0xFF) as f32;

        let fg_r = ((foreground >> 16) & 0xFF) as f32;
        let fg_g = ((foreground >> 8) & 0xFF) as f32;
        let fg_b = (foreground & 0xFF) as f32;

        let r = (fg_r * alpha + bg_r * inv_alpha) as u32;
        let g = (fg_g * alpha + bg_g * inv_alpha) as u32;
        let b = (fg_b * alpha + bg_b * inv_alpha) as u32;

        0xFF000000 | (r << 16) | (g << 8) | b
    }

    // Antialiased circle drawing with distance-based alpha
    fn draw_circle_fast(&mut self, cx: i32, cy: i32, radius: i32, color: Color) {
        let width = self.inner_size.width as i32;
        let height = self.inner_size.height as i32;

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        let radius_f = radius as f32;
        let color_u32 = color as u32;

        // Early bounds check with antialiasing margin
        let aa_radius = radius + 1;
        if cx + aa_radius < 0
            || cx - aa_radius >= width
            || cy + aa_radius < 0
            || cy - aa_radius >= height
        {
            return;
        }

        // Optimize bounds for the loop with antialiasing
        let start_y = (-aa_radius).max(-cy);
        let end_y = aa_radius.min(height - cy - 1);
        let start_x = (-aa_radius).max(-cx);
        let end_x = aa_radius.min(width - cx - 1);

        for dy in start_y..=end_y {
            for dx in start_x..=end_x {
                let distance = ((dx * dx + dy * dy) as f32).sqrt();

                if distance <= radius_f + 1.0 {
                    let x = cx + dx;
                    let y = cy + dy;

                    // Ensure x and y are within bounds before calculating index
                    if x >= 0 && x < width && y >= 0 && y < height {
                        let idx = (y * width + x) as usize;

                        let alpha = if distance <= radius_f {
                            1.0 // Full opacity inside the circle
                        } else {
                            // Antialiasing: fade out at the edge
                            1.0 - (distance - radius_f)
                        };

                        if alpha > 0.0 {
                            let current_color = self.pixels[idx]; // Read from self.pixels

                            let r = (((color_u32 >> 16) & 0xFF) as f32 * alpha
                                + ((current_color >> 16) & 0xFF) as f32 * (1.0 - alpha))
                                as u32;
                            let g = (((color_u32 >> 8) & 0xFF) as f32 * alpha
                                + ((current_color >> 8) & 0xFF) as f32 * (1.0 - alpha))
                                as u32;
                            let b = ((color_u32 & 0xFF) as f32 * alpha
                                + (current_color & 0xFF) as f32 * (1.0 - alpha))
                                as u32;

                            self.pixels[idx] = 0xFF000000 | (r << 16) | (g << 8) | b; // Write to self.pixels

                            min_x = min_x.min(x);
                            max_x = max_x.max(x);
                            min_y = min_y.min(y);
                            max_y = max_y.max(y);
                        }
                    }
                }
            }
        }

        if min_x <= max_x && min_y <= max_y {
            let rect = Rect {
                x: min_x as u32,
                y: min_y as u32,
                width: NonZeroU32::new((max_x - min_x + 1) as u32).unwrap(),
                height: NonZeroU32::new((max_y - min_y + 1) as u32).unwrap(),
            };

            // Only present the damaged region
            if let Some(surface) = self.surface.as_mut() {
                if let Ok(mut buffer) = surface.buffer_mut() {
                    let width = self.inner_size.width as usize;
                    // Copy only the affected region from self.pixels to the buffer
                    for y in rect.y..(rect.y + rect.height.get()) {
                        let src_start = (y * width as u32 + rect.x) as usize;
                        let src_end = (y * width as u32 + rect.x + rect.width.get()) as usize;
                        let dest_start = (y * width as u32 + rect.x) as usize;
                        buffer[dest_start..src_end]
                            .copy_from_slice(&self.pixels[src_start..src_end]);
                    }
                    let _ = buffer.present_with_damage(&[rect]);
                }
            }
        }
    }

    // Draw interpolated line between two points using Bresenham algorithm with antialiasing
    fn draw_interpolated_line(&mut self, x0: i32, y0: i32, x1: i32, y1: i32, color: Color) {
        let width = self.inner_size.width as i32;
        let height = self.inner_size.height as i32;

        let mut points = Vec::with_capacity(256); // Pre-allocate reasonable capacity

        points.clear();

        let dx = (x1 - x0).abs();
        let dy = -(y1 - y0).abs();

        let sx = if x0 < x1 { 1 } else { -1 };
        let sy = if y0 < y1 { 1 } else { -1 };

        let mut err = dx + dy;
        let mut x = x0;
        let mut y = y0;

        loop {
            points.push((x, y));

            if x == x1 && y == y1 {
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

        let mut min_x = i32::MAX;
        let mut max_x = i32::MIN;
        let mut min_y = i32::MAX;
        let mut max_y = i32::MIN;

        let radius = if self.draw_state == DrawState::Erasing {
            self.erase_brush_radius
        } else {
            self.draw_brush_radius
        };

        let radius_f = radius as f32;
        let color_u32 = color as u32;
        let aa_radius = radius + 1;

        // Draw antialiased circles at each point along the line
        for &(px, py) in &points {
            // Bounds check for the circle center with antialiasing margin
            if px + aa_radius < 0
                || px - aa_radius >= width
                || py + aa_radius < 0
                || py - aa_radius >= height
            {
                continue;
            }

            // Optimize bounds for the circle with antialiasing
            let start_dy = (-aa_radius).max(-py);
            let end_dy = aa_radius.min(height - py - 1);
            let start_dx = (-aa_radius).max(-px);
            let end_dx = aa_radius.min(width - px - 1);

            for dy in start_dy..=end_dy {
                for dx in start_dx..=end_dx {
                    let distance = ((dx * dx + dy * dy) as f32).sqrt();

                    if distance <= radius_f + 1.0 {
                        let x = px + dx;
                        let y = py + dy;

                        // Ensure x and y are within bounds before calculating index
                        if x >= 0 && x < width && y >= 0 && y < height {
                            let idx = (y * width + x) as usize;

                            let alpha = if distance <= radius_f {
                                1.0 // Full opacity inside the circle
                            } else {
                                // Antialiasing: fade out at the edge
                                1.0 - (distance - radius_f)
                            };

                            if alpha > 0.0 {
                                let current_color = self.pixels[idx]; // Read from self.pixels

                                let r = (((color_u32 >> 16) & 0xFF) as f32 * alpha
                                    + ((current_color >> 16) & 0xFF) as f32 * (1.0 - alpha))
                                    as u32;
                                let g = (((color_u32 >> 8) & 0xFF) as f32 * alpha
                                    + ((current_color >> 8) & 0xFF) as f32 * (1.0 - alpha))
                                    as u32;
                                let b = ((color_u32 & 0xFF) as f32 * alpha
                                    + (current_color & 0xFF) as f32 * (1.0 - alpha))
                                    as u32;

                                self.pixels[idx] = 0xFF000000 | (r << 16) | (g << 8) | b; // Write to self.pixels

                                min_x = min_x.min(x);
                                max_x = max_x.max(x);
                                min_y = min_y.min(y);
                                max_y = max_y.max(y);
                            }
                        }
                    }
                }
            }
        }

        if min_x <= max_x && min_y <= max_y {
            let rect = Rect {
                x: min_x as u32,
                y: min_y as u32,
                width: NonZeroU32::new((max_x - min_x + 1) as u32).unwrap(),
                height: NonZeroU32::new((max_y - min_y + 1) as u32).unwrap(),
            };

            // Only present the damaged region
            if let Some(surface) = self.surface.as_mut() {
                if let Ok(mut buffer) = surface.buffer_mut() {
                    let width = self.inner_size.width as usize;
                    // Copy only the affected region from self.pixels to the buffer
                    for y in rect.y..(rect.y + rect.height.get()) {
                        let src_start = (y * width as u32 + rect.x) as usize;
                        let src_end = (y * width as u32 + rect.x + rect.width.get()) as usize;
                        let dest_start = (y * width as u32 + rect.x) as usize;
                        buffer[dest_start..src_end]
                            .copy_from_slice(&self.pixels[src_start..src_end]);
                    }
                    let _ = buffer.present_with_damage(&[rect]);
                }
            }
        }
    }
}

impl Default for DrawOnScreen {
    fn default() -> Self {
        Self {
            window: None,
            context: None,
            surface: None,

            inner_size: PhysicalSize::new(0, 0),

            pixels: vec![], // Initialize as empty, will be sized on resume
            position: None,
            last_position: None,

            undo_stack: Vec::new(),
            redo_stack: Vec::new(),

            is_control_key_pressed: false,

            pointer_color: Color::White,
            draw_state: DrawState::Idle,
            draw_brush_radius: 1, // Default brush size
            erase_brush_radius: 3,
        }
    }
}

impl ApplicationHandler for DrawOnScreen {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        println!("Resumed");

        let window_attributes = Window::default_attributes()
            .with_title("Draw On Screen - Optimized")
            .with_cursor(CursorIcon::Pointer);

        let window = event_loop.create_window(window_attributes).unwrap();

        let window = Rc::new(window);

        let context = Context::new(window.clone()).unwrap();

        let surface = Surface::new(&context, window.clone()).unwrap();

        self.window = Some(window.clone());
        self.context = Some(context);
        self.surface = Some(surface);
        self.inner_size = window.inner_size();

        // Initialize pixels vector with the correct size and black color
        self.pixels =
            vec![Color::Black as u32; (self.inner_size.width * self.inner_size.height) as usize];

        window.request_redraw();
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        match event {
            WindowEvent::ModifiersChanged(modifiers) => {
                self.is_control_key_pressed = modifiers.state().control_key()
            }
            WindowEvent::CloseRequested => {
                println!("Window closed");
                event_loop.exit();
            }
            WindowEvent::KeyboardInput { event, .. } => {
                if event.state == ElementState::Pressed && !event.repeat {
                    use winit::keyboard::ModifiersState;

                    match event.physical_key {
                        PhysicalKey::Code(KeyCode::Digit1) => {
                            self.pointer_color = Color::Red;
                        }
                        PhysicalKey::Code(KeyCode::Digit2) => {
                            self.pointer_color = Color::Green;
                        }
                        PhysicalKey::Code(KeyCode::Digit3) => {
                            self.pointer_color = Color::Blue;
                        }
                        PhysicalKey::Code(KeyCode::Digit0) => {
                            self.pointer_color = Color::White;
                        }
                        PhysicalKey::Code(KeyCode::Equal)
                        | PhysicalKey::Code(KeyCode::NumpadAdd) => {
                            if self.draw_state == DrawState::Erasing {
                                self.erase_brush_radius = (self.erase_brush_radius + 1).min(50); // Max erasing size
                            } else {
                                self.draw_brush_radius = (self.draw_brush_radius + 1).min(20); // Max drawing size
                            }
                        }
                        PhysicalKey::Code(KeyCode::Minus)
                        | PhysicalKey::Code(KeyCode::NumpadSubtract) => {
                            if self.draw_state == DrawState::Erasing {
                                self.erase_brush_radius = (self.erase_brush_radius - 1).max(1);
                            } else {
                                self.draw_brush_radius = (self.draw_brush_radius - 1).max(1);
                            }
                        }

                        PhysicalKey::Code(KeyCode::Backspace) => {
                            // Clear screen
                            self.save_state();
                            self.pixels
                                .iter_mut()
                                .for_each(|pixel| *pixel = Color::Black as u32); // Clear self.pixels
                            if let Some(window) = &self.window {
                                window.request_redraw(); // Request redraw to show cleared screen
                            }
                        }

                        PhysicalKey::Code(KeyCode::KeyZ) => {
                            if self.is_control_key_pressed {
                                if let Some(last) = self.undo_stack.pop() {
                                    self.redo_stack.push(self.pixels.clone()); // Push current state to redo
                                    self.restore_state(last);
                                }
                            }
                        }

                        PhysicalKey::Code(KeyCode::KeyR) => {
                            if self.is_control_key_pressed {
                                if let Some(next) = self.redo_stack.pop() {
                                    self.undo_stack.push(self.pixels.clone()); // Push current state to undo
                                    self.restore_state(next);
                                }
                            }
                        }

                        _ => {}
                    }
                }
            }
            WindowEvent::Resized(size) => {
                let PhysicalSize { width, height } = size;

                if let Some(surface) = &mut self.surface {
                    surface
                        .resize(
                            NonZeroU32::new(width).unwrap(),
                            NonZeroU32::new(height).unwrap(),
                        )
                        .unwrap();

                    let old_width = self.inner_size.width;
                    let old_height = self.inner_size.height;

                    let new_width = width;
                    let new_height = height;

                    // Create a new pixels buffer for the new size, initialized to black
                    let mut new_pixels =
                        vec![Color::Black as u32; (new_width * new_height) as usize];

                    let copy_width = old_width.min(new_width);
                    let copy_height = old_height.min(new_height);

                    // Copy existing pixels to the new buffer
                    for y in 0..copy_height {
                        for x in 0..copy_width {
                            let old_idx = (y * old_width + x) as usize;
                            let new_idx = (y * new_width + x) as usize;

                            // Ensure indices are within bounds of old and new pixel buffers
                            if old_idx < self.pixels.len() && new_idx < new_pixels.len() {
                                new_pixels[new_idx] = self.pixels[old_idx];
                            }
                        }
                    }

                    // Update self.pixels with the new, resized content
                    self.pixels = new_pixels;
                    self.inner_size = size;

                    // --- FIX: Clear undo/redo stacks on resize to prevent size mismatches ---
                    self.undo_stack.clear();
                    self.redo_stack.clear();
                    // ---------------------------------------------------------------------

                    // Request a redraw to push the new self.pixels to softbuffer
                    if let Some(window) = &self.window {
                        window.request_redraw();
                    }
                }
            }
            WindowEvent::CursorMoved { position, .. } => {
                self.position = Some((position.x as i32, position.y as i32));
                // This block remains mostly the same, handling continuous drawing
                if self.draw_state == DrawState::Idle {
                    self.last_position = None;
                    return;
                }

                if self.last_position.is_none() {
                    self.save_state(); // save state at the start of a new stroke
                }

                let current_pos = (position.x as i32, position.y as i32);
                let color = if self.draw_state == DrawState::Erasing {
                    Color::Black
                } else {
                    self.pointer_color
                };

                let current_brush_radius = if self.draw_state == DrawState::Erasing {
                    self.erase_brush_radius
                } else {
                    self.draw_brush_radius
                };

                if let Some(last_pos) = self.last_position {
                    let dx = current_pos.0 - last_pos.0;
                    let dy = current_pos.1 - last_pos.1;
                    let distance_sq = dx * dx + dy * dy;

                    // Only draw an interpolated line if the mouse moved significantly
                    if distance_sq > (current_brush_radius * current_brush_radius / 2) as i32 {
                        self.draw_interpolated_line(
                            last_pos.0,
                            last_pos.1,
                            current_pos.0,
                            current_pos.1,
                            color,
                        );
                    } else {
                        // If movement is small, just draw a circle at the current position
                        // This helps fill small gaps and acts as the "click" drawing
                        self.draw_circle_fast(
                            current_pos.0,
                            current_pos.1,
                            current_brush_radius,
                            color,
                        );
                    }
                } else {
                    // First point of a new stroke (or a single click)
                    self.draw_circle_fast(
                        current_pos.0,
                        current_pos.1,
                        current_brush_radius,
                        color,
                    );
                }

                self.last_position = Some(current_pos);
            }
            WindowEvent::MouseWheel { delta, .. } => match delta {
                winit::event::MouseScrollDelta::LineDelta(_, y) => {
                    if y > 0.0 {
                        if self.draw_state == DrawState::Erasing {
                            self.erase_brush_radius = (self.erase_brush_radius + 1).min(50);
                        } else {
                            self.draw_brush_radius = (self.draw_brush_radius + 1).min(20);
                        }
                    } else if y < 0.0 {
                        if self.draw_state == DrawState::Erasing {
                            self.erase_brush_radius = (self.erase_brush_radius - 1).max(1);
                        } else {
                            self.draw_brush_radius = (self.draw_brush_radius - 1).max(1);
                        }
                    }
                }
                winit::event::MouseScrollDelta::PixelDelta(pos) => {
                    if pos.y > 0.0 {
                        if self.draw_state == DrawState::Erasing {
                            self.erase_brush_radius = (self.erase_brush_radius + 1).min(50);
                        } else {
                            self.draw_brush_radius = (self.draw_brush_radius + 1).min(20);
                        }
                    } else if pos.y < 0.0 {
                        if self.draw_state == DrawState::Erasing {
                            self.erase_brush_radius = (self.erase_brush_radius - 1).max(1);
                        } else {
                            self.draw_brush_radius = (self.draw_brush_radius - 1).max(1);
                        }
                    }
                }
            },
            WindowEvent::MouseInput { state, button, .. } => {
                if state == ElementState::Pressed {
                    if let Some(window) = &self.window {
                        // Get the current cursor position when the mouse button is pressed
                        if let Some(cursor_position) = self.position {
                            let current_pos = (cursor_position.0 as i32, cursor_position.1 as i32);
                            let color = if button == MouseButton::Right {
                                Color::Black
                            } else {
                                self.pointer_color
                            };
                            let current_brush_radius = if button == MouseButton::Right {
                                self.erase_brush_radius
                            } else {
                                self.draw_brush_radius
                            };

                            // Save state before drawing the initial dot
                            self.save_state();

                            // Draw a circle at the clicked position immediately
                            self.draw_circle_fast(
                                current_pos.0,
                                current_pos.1,
                                current_brush_radius,
                                color,
                            );

                            self.last_position = Some(current_pos);
                        }

                        match button {
                            MouseButton::Left => {
                                self.draw_state = DrawState::Drawing;
                            }
                            MouseButton::Right => {
                                self.draw_state = DrawState::Erasing;
                            }
                            _ => {}
                        }
                    }
                } else {
                    self.draw_state = DrawState::Idle;
                    self.last_position = None;
                }
            }
            WindowEvent::RedrawRequested => {
                // This is where you draw your `self.pixels` to the `softbuffer`
                if let Some(surface) = self.surface.as_mut() {
                    if let Ok(mut buffer) = surface.buffer_mut() {
                        let size = (self.inner_size.width * self.inner_size.height) as usize;
                        if buffer.len() == size {
                            buffer.copy_from_slice(&self.pixels); // Copy all pixels from your buffer
                            let _ = buffer.present(); // Full present
                        } else {
                            // This might happen if `resize` is called but `RedrawRequested` comes before the new buffer is ready.
                            // In this case, we re-initialize the buffer to black.
                            for pixel in buffer.iter_mut() {
                                *pixel = Color::Black as u32;
                            }
                            let _ = buffer.present();
                        }
                    }
                }
            }
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
