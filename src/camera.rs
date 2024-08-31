use winit::keyboard::{KeyCode, PhysicalKey};
use winit::event::{ElementState, KeyEvent, MouseScrollDelta, WindowEvent};

#[rustfmt::skip]
pub const OPENGL_TO_WGPU_MATRIX: cgmath::Matrix4<f32> = cgmath::Matrix4::new(
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 0.5, 0.5,
    0.0, 0.0, 0.0, 1.0,
);


fn calculate_screen_space(pos: cgmath::Vector4<f32>, size: winit::dpi::PhysicalSize<u32>) -> cgmath::Vector2<f32> {
    let x = (size.width as f32 * ((pos.x / pos.w) + 1.0)) / 2.0;
    let y = (size.height as f32 * ((pos.y / pos.w) - 1.0)) / -2.0;

    cgmath::Vector2 {
        x,
        y,
    }
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
        
        return OPENGL_TO_WGPU_MATRIX * proj * view;
    }

    pub fn world_to_screen_space(&self, pos: cgmath::Vector3<f32>, size: winit::dpi::PhysicalSize<u32>) -> cgmath::Vector2<f32> {
        // convert from world space to camera space
        let pos = self.build_view_projection_matrix() * cgmath::vec4(pos.x, pos.y, pos.z, 1.0);
        // convert from camera space to screen space
        calculate_screen_space(pos, size)
        
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
    is_up_pressed: bool,
    is_down_pressed: bool,
    is_left_pressed: bool,
    is_right_pressed: bool,
    scroll: f32,
}

impl CameraController {
    pub fn new(speed: f32) -> Self {
        Self {
            speed,
            is_up_pressed: false,
            is_down_pressed: false,
            is_left_pressed: false,
            is_right_pressed: false,
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
                    _ => false,
                }
            },
            _ => false,
        }
    }

    pub fn update_camera(&mut self, camera: &mut Camera) {
        use cgmath::InnerSpace;
        let forward = camera.target - camera.eye;
        let forward_norm = forward.normalize();
        let forward_mag = forward.magnitude();
        
        let change = forward_norm * forward_mag * self.speed * self.scroll;
        let next_power_of_two = ((camera.eye.z + change.z) as u32).checked_next_power_of_two(); 

        if self.scroll > 0.0 && camera.eye.z >= 1.0 {
            camera.eye += change;
            self.scroll = 0.0;
        }
        if self.scroll < 0.0 && next_power_of_two.is_some() {
            camera.eye += change;
            self.scroll = 0.0;
        }
        if self.is_up_pressed {
            camera.eye.y += 0.01 * camera.eye.z;
            camera.target.y += 0.01 * camera.eye.z;
        }
        if self.is_down_pressed {
            camera.eye.y -= 0.01 * camera.eye.z;
            camera.target.y -= 0.01 * camera.eye.z;
        }
        if self.is_left_pressed {
            camera.eye.x -= 0.01 * camera.eye.z;
            camera.target.x -= 0.01 * camera.eye.z;
        }
        if self.is_right_pressed {
            camera.eye.x += 0.01 * camera.eye.z;
            camera.target.x += 0.01 * camera.eye.z;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_calculate_screen_space() {
        let size = winit::dpi::PhysicalSize::new(256 as u32, 256 as u32);
        
        let pos0 = cgmath::Vector4 { x: 0.0, y: 0.0, z: 0.0, w: 1.0 };
        assert_eq!(calculate_screen_space(pos0, size), cgmath::vec2(128.0, 128.0));

        let pos1 = cgmath::Vector4 { x: 1.0, y: -1.0, z: 0.0, w: 1.0 };
        assert_eq!(calculate_screen_space(pos1, size), cgmath::vec2(256.0, 256.0));

        let pos2 = cgmath::Vector4 { x: -1.0, y: 1.0, z: 0.0, w: 1.0 };
        assert_eq!(calculate_screen_space(pos2, size), cgmath::vec2(0.0, 0.0));
    }
}
