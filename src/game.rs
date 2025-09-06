use crate::{
    input::{Event, Key},
    renderer::Renderer,
};
use cgmath::{perspective, Deg, Matrix4, Point3, Vector3};

pub struct Game {
    rotation_x: f32,
    rotation_y: f32,
    mouse_dragging: bool,

    // Anchor state for drag
    drag_start_x: f32,
    drag_start_y: f32,
    start_rotation_x: f32,
    start_rotation_y: f32,

    // Animation time for advanced effects
    time: f32,
}

impl Game {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        log::info!("Initializing game state");
        Ok(Self {
            rotation_x: 0.0,
            rotation_y: 0.0,
            mouse_dragging: false,
            drag_start_x: 0.0,
            drag_start_y: 0.0,
            start_rotation_x: 0.0,
            start_rotation_y: 0.0,
            time: 0.0,
        })
    }

    pub fn handle_event(&mut self, event: Event) {
        match event {
            Event::MouseDown {
                x,
                y,
                width: _,
                height: _,
            } => {
                self.mouse_dragging = true;
                self.drag_start_x = x;
                self.drag_start_y = y;
                self.start_rotation_x = self.rotation_x;
                self.start_rotation_y = self.rotation_y;
            }
            Event::MouseUp { .. } => {
                self.mouse_dragging = false;
            }

            Event::MouseMove {
                x,
                y,
                width,
                height,
            } => {
                if self.mouse_dragging {
                    let dx = (x - self.drag_start_x) / width;
                    let dy = (y - self.drag_start_y) / height;

                    self.rotation_x = self.start_rotation_x + dy * std::f32::consts::PI;
                    self.rotation_y = self.start_rotation_y + dx * std::f32::consts::TAU;
                }
            }
            Event::TouchDown {
                id: _,
                x,
                y,
                width: _,
                height: _,
            } => {
                self.mouse_dragging = true;
                self.drag_start_x = x;
                self.drag_start_y = y;
                self.start_rotation_x = self.rotation_x;
                self.start_rotation_y = self.rotation_y;
            }
            Event::TouchMove {
                id: _,
                x,
                y,
                width,
                height,
            } => {
                if self.mouse_dragging {
                    let dx = (x - self.drag_start_x) / width;
                    let dy = (y - self.drag_start_y) / height;

                    self.rotation_x = self.start_rotation_x + dy * std::f32::consts::PI;
                    self.rotation_y = self.start_rotation_y + dx * std::f32::consts::TAU;
                }
            }
            Event::TouchUp { id: _ } => {
                self.mouse_dragging = false;
            }

            Event::KeyDown { key } => match key {
                Key::R => {
                    self.rotation_x = 0.0;
                    self.rotation_y = 0.0;
                }
                Key::Space => {}
                _ => {}
            },
            Event::Quit => {}
        }
    }

    pub fn update(&mut self, delta_time: f32) {
        self.time += delta_time;

        if !self.mouse_dragging {
            self.rotation_y += 0.2 * delta_time;
        }
    }

    pub fn render(&self, renderer: &mut Renderer, width: u32, height: u32) {
        let model = Matrix4::from_angle_x(Deg(self.rotation_x * 57.2958))
            * Matrix4::from_angle_y(Deg(self.rotation_y * 57.2958));

        let view = Matrix4::look_at_rh(
            Point3::new(0.0, 0.0, 3.0),
            Point3::new(0.0, 0.0, 0.0),
            Vector3::new(0.0, 1.0, 0.0),
        );

        let aspect = (width as f32).max(1.0) / (height as f32).max(1.0);
        let projection = perspective(Deg(45.0), aspect, 0.1, 100.0);

        let mvp = projection * view * model;
        renderer.render_triangle(&mvp);
    }

    pub fn get_time(&self) -> f32 {
        self.time
    }
}
