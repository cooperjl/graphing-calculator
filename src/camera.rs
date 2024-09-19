use winit::dpi::{PhysicalPosition, PhysicalSize};
use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event::{ElementState, KeyEvent, MouseButton, MouseScrollDelta, WindowEvent};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);


fn calculate_screen_space(pos: cgmath::Vector2<f32>, size: PhysicalSize<u32>) -> cgmath::Vector2<f32> {
    let x = (size.width as f32 * (pos.x + 1.0)) / 2.0;
    let y = (size.height as f32 * (pos.y - 1.0)) / -2.0;

    cgmath::Vector2 { x, y }
}

fn normalise_screen_space(pos: cgmath::Vector2<f32>, size: PhysicalSize<u32>) -> cgmath::Vector2<f32> {
    let x = ((2.0 / size.width as f32) * pos.x) - 1.0;
    let y = ((-2.0 / size.height as f32) * pos.y) + 1.0;

    cgmath::Vector2 { x, y }
}

pub struct Camera {
    pub eye: cgmath::Point3<f32>,
    pub target: cgmath::Point3<f32>,
    pub up: cgmath::Vector3<f32>,
    pub aspect: f32,
    pub fovy: f32,
    pub znear: f32,
    pub zfar: f32,
}

impl Camera {
    pub fn build_view_projection_matrix(&self) -> cgmath::Matrix4<f32> {
        let view = cgmath::Matrix4::look_at_rh(self.eye, self.target, self.up);
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);
        
        OPENGL_TO_WGPU_MATRIX * proj * view
    }

    fn build_proj_matrix(&self) -> cgmath::Matrix4<f32> {
        let proj = cgmath::perspective(cgmath::Deg(self.fovy), self.aspect, self.znear, self.zfar);

        OPENGL_TO_WGPU_MATRIX * proj
    }

    pub fn world_to_screen_space(&self, pos: cgmath::Vector3<f32>, size: PhysicalSize<u32>) -> cgmath::Vector2<f32> {
        // convert from world space to clip space
        let clip_pos = self.build_view_projection_matrix() * cgmath::vec4(pos.x, pos.y, pos.z, 1.0);
        // convert from clip space to normalised space
        let normal_pos = cgmath::Vector2 { x: clip_pos.x / clip_pos.w, y: clip_pos.y / clip_pos.w };
        // convert from normalised space to screen space
        calculate_screen_space(normal_pos, size)
    }

    pub fn screen_to_view_space(&self, pos: cgmath::Vector2<f32>, size: PhysicalSize<u32>) -> cgmath::Vector2<f32> {
        use cgmath::SquareMatrix;
        // convert from screen space to normalised space
        let normal_pos = normalise_screen_space(pos, size);
        // convert from normalised space to view space
        let pos = self.build_proj_matrix().invert().unwrap() * cgmath::vec4(normal_pos.x, normal_pos.y, 0.0, 1.0);

        cgmath::Vector2 { x: pos.x * 1.5, y: pos.y * 1.5 }
    }
        
    /// Calculates the distance from the origin of this transformation to the cursor_location and
    /// adjusts the pan/translation in the x and y axes.
    pub fn adjust_pan_with_cursor_position(&mut self, cursor_location: PhysicalPosition<f32>, origin: cgmath::Vector2<f32>, modifier: f32, size: PhysicalSize<u32>) {
        // calculate view space positions for the cursor and origin
        let cursor_view = self.screen_to_view_space(cgmath::vec2(cursor_location.x, cursor_location.y), size);
        let origin_view = self.screen_to_view_space(origin, size);
        // calculate the distance from the cursor to the origin
        let distance = cgmath::vec2(cursor_view.x - origin_view.x, cursor_view.y - origin_view.y);
        // scale the distance with the zoom factor
        let change = cgmath::vec3(distance.x * self.eye.z, distance.y * self.eye.z, 0.0);

        // attempt to snap onto the position when small enough for a fractional modifier as it
        // never reaches the position even though it ideally would TODO improve this
        let modnew = if modifier.abs()/4.0 >= (distance.x.abs().powf(2.0)*distance.y.abs().powf(2.0)).sqrt() {
            modifier.signum()
        } else {
            modifier
        };

        // apply a modifier, mainly used to decide which direction the change should be in
        self.eye += change * modnew;
        self.target += change * modnew;

    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct CameraUniform {
    view_proj: [[f32; 4]; 4],
}

impl CameraUniform {
    pub fn new() -> Self {
        use cgmath::SquareMatrix;
        Self {
            view_proj: cgmath::Matrix4::identity().into(),
        }
    }

    pub fn update_view_proj(&mut self, camera: &Camera) {
        self.view_proj = camera.build_view_projection_matrix().into();
    }
}

pub struct CameraController {
    speed: f32,
    cursor_location: PhysicalPosition<f32>,
    mouse_clicked_at: Option<PhysicalPosition<f32>>,
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    is_mouse_pressed: bool,
    is_mouse_released: bool,
    scroll: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            cursor_location: PhysicalPosition { x: 0.0, y: 0.0 },
            mouse_clicked_at: None,
            is_up_pressed: false,
            is_down_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
            is_mouse_pressed: false,
            is_mouse_released: true,
            scroll: 0.0,
        }
    }

    pub fn process_events(&mut self, event: &WindowEvent) -> bool {
        match event {
            WindowEvent::KeyboardInput {
                event:
                    KeyEvent {
                        state,
                        physical_key: PhysicalKey::Code(keycode),
                        ..
                    },
                    ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                match keycode {
                    KeyCode::KeyW | KeyCode::ArrowUp => {
                        self.is_up_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyS | KeyCode::ArrowDown => {
                        self.is_down_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyA | KeyCode::ArrowLeft => {
                        self.is_left_pressed = is_pressed;
                        true
                    }
                    KeyCode::KeyD | KeyCode::ArrowRight => {
                        self.is_right_pressed = is_pressed;
                        true
                    }
                    _ => false,
                }
            },
            WindowEvent::MouseWheel {
                delta,
                ..
            } => {
                match delta {
                    MouseScrollDelta::LineDelta(_x, y) => {
                        self.scroll = *y;
                        true

                    }
                    MouseScrollDelta::PixelDelta(position) => {
                        self.scroll = position.y as f32;
                        true
                    }
                }
            },
            WindowEvent::CursorMoved {
                position,
                ..
            } => {
                self.cursor_location.x = position.x as f32;
                self.cursor_location.y = position.y as f32;
                true
            },
            WindowEvent::MouseInput {
                state,
                button,
                ..
            } => {
                let is_pressed = *state == ElementState::Pressed;
                let is_released = *state == ElementState::Released;
                if let MouseButton::Left = button {
                    self.is_mouse_pressed = is_pressed;
                    self.is_mouse_released = is_released;
                }
                true
            }
            _ => false,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera, size: PhysicalSize<u32>) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();
        
        let zoom_change = forward_norm * forward_mag * self.speed * self.scroll;
        let next_power_of_two = ((camera.eye.z + zoom_change.z) as u32).checked_next_power_of_two(); 

        let not_at_scroll_min = self.scroll > 0.0 && camera.eye.z >= 1.0;
        let not_at_scroll_max = self.scroll < 0.0 && next_power_of_two.is_some();

        if not_at_scroll_min || not_at_scroll_max {
            camera.eye += zoom_change;
            
            let origin = camera.world_to_screen_space(cgmath::vec3(0.0, 0.0, 0.0), size);
            camera.adjust_pan_with_cursor_position(self.cursor_location, origin, self.scroll * 0.25, size);
            self.scroll = 0.0;
        }

        if self.is_mouse_pressed {
            if self.mouse_clicked_at.is_none() {
                // record click location
                self.mouse_clicked_at = Some(self.cursor_location);
            } else {
                // moving mouse_clicked_at to cursor_location
                let mouse_clicked_at_pos = cgmath::vec2(self.mouse_clicked_at.unwrap().x, self.mouse_clicked_at.unwrap().y);
                camera.adjust_pan_with_cursor_position(self.cursor_location, mouse_clicked_at_pos, -1.0, size);
                // update the click location now that the movement has occurred
                self.mouse_clicked_at = Some(self.cursor_location);
            }
        }
        if self.is_mouse_released {
            self.mouse_clicked_at = None;
        }

        if self.is_up_pressed {
            camera.eye.y += self.speed * camera.eye.z;
            camera.target.y += self.speed * camera.eye.z;
        }
        if self.is_down_pressed {
            camera.eye.y -= self.speed * camera.eye.z;
            camera.target.y -= self.speed * camera.eye.z;
        }
        if self.is_left_pressed {
            camera.eye.x -= self.speed * camera.eye.z;
            camera.target.x -= self.speed * camera.eye.z;
        }
        if self.is_right_pressed {
            camera.eye.x += self.speed * camera.eye.z;
            camera.target.x += self.speed * camera.eye.z;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_screen_space() {
        let size = PhysicalSize::new(256, 256);
        
        let pos = cgmath::Vector2 { x: 0.0, y: 0.0 };
        assert_eq!(calculate_screen_space(pos, size), cgmath::vec2(128.0, 128.0));

        let pos = cgmath::Vector2 { x: 1.0, y: -1.0 };
        assert_eq!(calculate_screen_space(pos, size), cgmath::vec2(256.0, 256.0));

        let pos = cgmath::Vector2 { x: -1.0, y: 1.0 };
        assert_eq!(calculate_screen_space(pos, size), cgmath::vec2(0.0, 0.0));
    }

    #[test]
    fn test_normalise_screen_space() {
        let size = PhysicalSize::new(256, 256);
        
        let pos = cgmath::Vector2 { x: 128.0, y: 128.0 };
        assert_eq!(normalise_screen_space(pos, size), cgmath::vec2(0.0, 0.0));

        let pos = cgmath::Vector2 { x: 256.0, y: 256.0 };
        assert_eq!(normalise_screen_space(pos, size), cgmath::vec2(1.0, -1.0));

        let pos = cgmath::Vector2 { x: 0.0, y: 0.0 };
        assert_eq!(normalise_screen_space(pos, size), cgmath::vec2(-1.0, 1.0));
    }
}
