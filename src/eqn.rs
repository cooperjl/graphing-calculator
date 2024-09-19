use cgmath::num_traits::Float;
use wgpu::{self, util::DeviceExt, include_wgsl};

use crate::vertex::{Color, ColorUniform, InstanceRaw, Vertex};
use crate::camera;

pub enum EquationType {
    Polynomial,
    Exponential, // TODO
    Trigonometric, // TODO
    Circle, // TODO
}

/// Returns two vertices a certain distance from a point that can be used to form a line.
///
/// Takes four inputs: the first point, the second point, the width of the square, and a bool
/// specifying whether initial or not. initial refers to whether the square points of p1 are wanted
/// or not. This is useful for beginning a line with the initial start point.
///
/// This function always needs the two points the line segment will be between, but only returns
/// two of the four vertices needed to avoid repeated vertices on lines.
fn square_points(p1: cgmath::Vector2<f32>, p2: cgmath::Vector2<f32>, width: f32, initial: bool) -> Vec<Vertex> {
    let theta = f32::atan2(p1.x - p2.x, p1.y - p2.y);
    let delta_x = f32::cos(theta) * width;
    let delta_y = f32::sin(theta) * width;

    if initial {
        vec![
            Vertex { position: [p1.x + delta_x, p1.y - delta_y, 0.0] },
            Vertex { position: [p1.x - delta_x, p1.y + delta_y, 0.0] },
        ]
    } else {
        vec![
            Vertex { position: [p2.x + delta_x, p2.y - delta_y, 0.0] },
            Vertex { position: [p2.x - delta_x, p2.y + delta_y, 0.0] },
        ]
    }
}

/// Returns the corresponding y value to the x value for a polynomial equation.
///
/// Takes x as an input, as well as a list of coefficients ordered from the largest order to the
/// smallest, including x^0.
fn polynomial_equation(x: f32, coeffs: &[f32]) -> f32 {
    coeffs.iter().rev().enumerate()
        .map(|(i, coeff)| coeff * x.powi(i as i32))
        .sum::<f32>()
        //.min(max_y)
        //.max(min_y)
}

pub struct Line {
    pub width: f32,
    pub coeffs: Vec<f32>,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    pub color_bind_group: wgpu::BindGroup,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
}

impl Line {
    pub fn new(device: &wgpu::Device, coeffs: Vec<f32>, width: f32, color: Color<f32>, color_bind_group_layout: &wgpu::BindGroupLayout) -> Self {
        let vertices = Vec::new();
        let indices = Vec::new();

        let vertex_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Equation Vertex Buffer"),
                size: 1000000, // TODO work this out properly
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let index_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Equation Index Buffer"),
                size: 1000000, // TODO work this out properly
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let color_uniform = ColorUniform::new(color);
        
        let color_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Color Buffer"),
                contents: bytemuck::cast_slice(&[color_uniform]),
                usage: wgpu::BufferUsages::UNIFORM,
            }
        );

        let color_bind_group = device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: color_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: color_buffer.as_entire_binding(),
                    }
                ],
                label: Some("Color Bind Group"),
            }
        );

        Self {
            width,
            coeffs,
            vertices,
            indices,
            color_bind_group,
            vertex_buffer,
            index_buffer,
        }
    }

    pub fn make_polynomial(&mut self, x_min: i32, x_max: i32) {
        self.indices = Vec::new();
        self.vertices = Vec::new();

        for (i, num) in (x_min*10..x_max*10).step_by(((x_min.abs() + x_max.abs()) as f32/20.0).ceil() as usize).enumerate() {
            let x1 = num as f32 / 10.0;
            let y1 = polynomial_equation(x1, self.coeffs.as_slice());
            let p1 = cgmath::vec2(x1, y1);

            let x2 = (num as f32 + 1.0) / 10.0;
            let y2 = polynomial_equation(x2, self.coeffs.as_slice());
            let p2 = cgmath::vec2(x2, y2);

            if i == 0 {
                self.vertices.append(&mut square_points(p1, p2, self.width, true));
            }

            self.next(i as u16 * 2, p1, p2);
        }
    }

    fn next(&mut self, offset: u16, p1: cgmath::Vector2<f32>, p2: cgmath::Vector2<f32>) {
        self.vertices.append(&mut square_points(p1, p2, self.width, false));
        self.indices.append(&mut [
            offset, offset+1, offset+3,
            offset+2, offset, offset+3,
        ].to_vec());
    }

    pub fn update_buffers(&mut self, queue: &wgpu::Queue) {
        let vertex_data = self.vertices.to_vec();
        let index_data = self.indices.to_vec();

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&index_data));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_polynomial_equation() {
        let coeffs = &[];
        assert_eq!(polynomial_equation(2.0, coeffs), 0.0);
        let coeffs = &[1.0, 4.0, 3.0, -1.0];
        assert_eq!(polynomial_equation(2.0, coeffs), 29.0);
        let coeffs = &[1.0, 0.0];
        assert_eq!(polynomial_equation(2.0, coeffs), 2.0);
    }
}
