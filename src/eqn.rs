use wgpu::{self, include_wgsl};
use cgmath::prelude::*;

use crate::vertex::{Vertex, Instance, InstanceRaw, Color};

pub enum EquationType {
    Linear, // TODO may remove as polynomial covers linear
    Polynomial,
    Exponential, // TODO
    Trigonometric, // TODO
    Circle, // TODO
}

pub struct Line {
    pub width: f32,
    pub offset: u16,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<u16>,
    head_point: cgmath::Vector2<f32>,
}

fn initial_square_points(p1: cgmath::Vector2<f32>, p2: cgmath::Vector2<f32>, width: f32) -> Vec<Vertex> {
    let theta = f32::atan2(p1.x - p2.x, p1.y - p2.y);
    let delta_x = f32::cos(theta) * width;
    let delta_y = f32::sin(theta) * width;

    vec![
        Vertex { position: [p1.x + delta_x, p1.y - delta_y, 0.0] },
        Vertex { position: [p1.x - delta_x, p1.y + delta_y, 0.0] },
        Vertex { position: [p2.x + delta_x, p2.y - delta_y, 0.0] },
        Vertex { position: [p2.x - delta_x, p2.y + delta_y, 0.0] },
    ]
    
}

fn next_square_points(p1: cgmath::Vector2<f32>, p2: cgmath::Vector2<f32>, width: f32) -> Vec<Vertex> {
    let theta = f32::atan2(p1.x - p2.x, p1.y - p2.y);
    let delta_x = f32::cos(theta) * width;
    let delta_y = f32::sin(theta) * width;

    vec![
        Vertex { position: [p2.x + delta_x, p2.y - delta_y, 0.0] },
        Vertex { position: [p2.x - delta_x, p2.y + delta_y, 0.0] },
    ]
}

impl Line {
    pub fn new(p1: cgmath::Vector2<f32>, p2: cgmath::Vector2<f32>, width: f32) -> Self {
        let vertices = initial_square_points(p1, p2, width);
        let indices = vec![
            0, 1, 3,
            2, 0, 3,
        ];

        // increases by 4/2 (=2)
        let offset = 2;

        Self {
            width,
            offset,
            vertices,
            indices,
            head_point: p2,
        }
    }

    fn next(&mut self, p: cgmath::Vector2<f32>) {
        self.vertices.append(&mut next_square_points(self.head_point, p, self.width));
        self.indices.append(&mut [
            self.offset, self.offset+1, self.offset+3,
            self.offset+2, self.offset, self.offset+3,
        ].to_vec());

        self.offset += 2;
        self.head_point = p;
    }
}

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
) -> wgpu::RenderPipeline {
    let shader = device.create_shader_module(shader);

    device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("Render Pipeline"),
        layout: Some(layout),
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: vertex_layouts,
            compilation_options: Default::default(),
        },
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format,
                blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
            compilation_options: Default::default(),
        }),
        primitive: wgpu::PrimitiveState {
            topology: wgpu::PrimitiveTopology::TriangleList,
            strip_index_format: None,
            front_face: wgpu::FrontFace::Ccw,
            cull_mode: Some(wgpu::Face::Back),
            polygon_mode: wgpu::PolygonMode::Fill,
            unclipped_depth: false,
            conservative: false,
        },
        depth_stencil: None,
        multisample: wgpu::MultisampleState {
            count: 1,
            mask: !0,
            alpha_to_coverage_enabled: false,
        },
        multiview: None,
        cache: None,
    })
}

pub struct Equation {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<Instance>,
    pub vertices: Vec<Vertex>,
    pub line: Line,
}

impl Equation {
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, format: wgpu::TextureFormat) -> Self {
        let render_pipeline = create_render_pipeline(
            device, 
            pipeline_layout, 
            format, 
            &[Vertex::desc(), InstanceRaw::desc()],
            include_wgsl!("eqn_shader.wgsl"),
        );

        // TODO buffers per equation
        let vertex_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Equation Vertex Buffer"),
                size: 1000000,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let index_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Equation Index Buffer"),
                size: 1000000,
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Equation Instance Buffer"),
                size: 80,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );
        
        let position = cgmath::Vector3 { x: 0.0, y: 0.0, z: 0.0 };
        let rotation = if position.is_zero() {
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
        };

        let color = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };

        let instance = Instance { position, rotation, color };
        let instances = vec![instance];

        let vertices = vec![];

        let p = cgmath::vec2(0.0, 0.0);
        let width = 0.025;
        
        // TODO we need multiple lines at once so this is temporary while it is just 1 line
        let line = Line::new(p, p, width);

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            instance_buffer,
            instances,
            vertices,
            line,
        }
    }

    fn polynomial_equation(&self, x: f32, coeffs: &[f32]) -> f32 {
        coeffs.iter().rev().enumerate()
            .map(|(i, coeff)| coeff * x.powi(i as i32))
            .sum()
    }

    pub fn make_cubic(&mut self) {
        // x^3 + 4x^2 + 3x - 1
        let coeffs = &[1.0, 4.0, 3.0, -1.0];

        for i in -200..200 {
            let x = i as f32 / 10.0;
            let y = self.polynomial_equation(x, coeffs);
            self.vertices.push(Vertex { position: [x, y, 0.0 ] });
        }
        let p1 = cgmath::vec2(self.vertices[0].position[0], self.vertices[0].position[1]);
        let p2 = cgmath::vec2(self.vertices[1].position[0], self.vertices[1].position[1]);

        self.line = Line::new(p1, p2, self.line.width);

        for (i, point) in self.vertices.iter().enumerate() {
            let p = cgmath::vec2(point.position[0], point.position[1]);
            // TODO replace with slice
            if i >= 2 {
                self.line.next(p);
            }
        }
    }
    


    pub fn update_equations(&mut self, queue: &wgpu::Queue) {
        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        let mut vertex_data: Vec<Vertex> = Vec::new();
        let mut index_data: Vec<u16> = Vec::new();

        for vertex in &self.line.vertices {
            vertex_data.push(*vertex);
        }
        for index in &self.line.indices {
            index_data.push(*index);
        }

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&vertex_data));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&index_data));
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data));
    }
}
