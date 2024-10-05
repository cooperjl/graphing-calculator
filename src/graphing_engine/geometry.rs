use wgpu::{self, util::DeviceExt};

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
}

impl Vertex {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float32x3,
                },
            ]
        }
    }
}

pub struct Color<T> {
    pub r: T,
    pub g: T,
    pub b: T,
    pub a: T,
}

impl<T: Copy> Color<T> {
    pub fn to_raw(&self) -> [T; 4] {
        [self.r, self.g, self.b, self.a]
    }
}

#[repr(C)]
#[derive(Debug, Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct ColorUniform {
    raw: [f32; 4],
}

impl ColorUniform {
    pub fn new(color: Color<f32>) -> Self {
        Self {
            raw: color.to_raw()
        }
    }
}

pub struct Instance {
    pub position: cgmath::Vector3<f32>,
    pub rotation: cgmath::Quaternion<f32>,
    pub color: Color<f32>,
}

#[repr(C)]
#[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
pub struct InstanceRaw {
    pub model: [[f32; 4]; 4],
    pub color: [f32; 4],
}

impl Instance {
    pub fn to_raw(&self) -> InstanceRaw {
        InstanceRaw {
            model: (cgmath::Matrix4::from_translation(self.position) * cgmath::Matrix4::from(self.rotation)).into(),
            color: self.color.to_raw(),
        }
    }
}

impl InstanceRaw {
    pub fn desc() -> wgpu::VertexBufferLayout<'static> {
        use std::mem;
        wgpu::VertexBufferLayout {
            array_stride: mem::size_of::<InstanceRaw>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Instance,
            attributes: &[
                wgpu::VertexAttribute {
                    offset: 0,
                    shader_location: 5,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 6,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 8]>() as wgpu::BufferAddress,
                    shader_location: 7,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 12]>() as wgpu::BufferAddress,
                    shader_location: 8,
                    format: wgpu::VertexFormat::Float32x4,
                },
                wgpu::VertexAttribute {
                    offset: mem::size_of::<[f32; 16]>() as wgpu::BufferAddress,
                    shader_location: 9,
                    format: wgpu::VertexFormat::Float32x4,
                },
            ],
        }
    }
}

pub struct Circle {
    pub radius: f32,
    pub segments: u16,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
}

impl Circle {
    pub fn new(radius: f32, segments: u16) -> Self {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<u16> = Vec::new();

        vertices.push(Vertex { position: [0.0, 0.0, 0.0] });
        indices.append(&mut [0, segments, 1].to_vec());

        for s in 0..segments {
            // trace the circle and place points along it
            let current_seg = (2.0 * std::f32::consts::PI) * (s as f32 / segments as f32);

            let x = radius * current_seg.cos();
            let y = radius * current_seg.sin();
            let z = 0.0;

            vertices.push(Vertex { position: [x, y, z] });
        }

        for i in 1..segments {
            indices.append(&mut [0, i, i+1].to_vec());
        }

        Self {
            radius,
            segments,
            vertices,
            indices,
        }
    }
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
/// Takes x as an input, as well as a list of coefficients ordered from the smallest order to the
/// largest, including x^0.
fn polynomial_equation(x: f32, coeffs: &[f32]) -> f32 {
    coeffs.iter().enumerate()
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
    pub fn new(device: &wgpu::Device,
        coeffs: Vec<f32>,
        width: f32, 
        color: Color<f32>, 
        color_bind_group_layout: &wgpu::BindGroupLayout
    ) -> Self {
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
        
        let step_size = (x_max.abs().saturating_add(x_min.saturating_abs()) as f32 / 40.0).ceil() as usize;
        let unit = 20;

        for (i, num) in (x_min.saturating_mul(unit)..x_max.saturating_mul(unit)).step_by(step_size).enumerate() {
            let x1: f32 = num as f32 / unit as f32;
            let y1 = polynomial_equation(x1, self.coeffs.as_slice());
            let p1 = cgmath::vec2(x1, y1);

            let x2 = (num as f32 + 1.0) / unit as f32;
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
        let coeffs = &[-1.0, 3.0, 4.0, 1.0];
        assert_eq!(polynomial_equation(2.0, coeffs), 29.0);
        let coeffs = &[0.0, 1.0];
        assert_eq!(polynomial_equation(2.0, coeffs), 2.0);
    }

    #[test]
    fn circle_vertices_on_circle() {
        use approx::relative_eq;

        let radius = 1.0;
        let circle = Circle::new(radius, 32);

        for vertex in circle.vertices {
            if vertex.position != [0.0, 0.0, 0.0] {
                let x_squared = vertex.position[0].powf(2.0);
                let y_squared = vertex.position[1].powf(2.0);
                let r_squared = radius.powf(2.0);
                
                assert!(relative_eq!(x_squared + y_squared, r_squared));
            }
        }
    }
}
