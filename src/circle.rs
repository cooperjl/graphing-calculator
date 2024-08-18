use wgpu::{self, include_wgsl, util::DeviceExt};
use cgmath::prelude::*;

use crate::vertex::{Vertex, Instance, InstanceRaw};
use crate::camera;

pub struct Circle {
    pub radius: f32,
    pub segments: u32,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<[u32; 3]>,
}

impl Circle {
    pub fn new(radius: f32, segments: u32) -> Self {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<[u32; 3]> = Vec::new();

        vertices.push(Vertex { position: [0.0, 0.0, 0.0] });
        indices.push([1, segments, 0]);

        for s in 0..segments {
            // we trace the circle and place points along it
            let current_seg = (2.0 * std::f32::consts::PI) * (s as f32 / segments as f32);

            let x = radius * current_seg.cos();
            let y = radius * current_seg.sin();
            let z = 0.0;

            vertices.push(Vertex { position: [x, y, z] });
        }

        for i in 1..segments {
            indices.push([i + 1, i, 0]);
        }

        Self {
            radius,
            segments,
            vertices,
            indices,
        }
    }
}

pub struct PointPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<Instance>,
    pub circle: Circle,
}

impl PointPipeline {
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, config: &wgpu::SurfaceConfiguration) -> Self {
        let shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
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
        });


        let circle = Circle::new(0.01, 32);

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&circle.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&circle.indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let num_indices = circle.segments * 6;

        let mut instances = vec![];

        let position = cgmath::Vector3 { x: 1.0, y: 1.0, z: 0.0 };

        let rotation = if position.is_zero() {
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
        };

        let color = [0.0, 0.0, 1.0, 1.0];

        instances.push(Instance {
            position,
            rotation,
            color,
        });

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            instance_buffer,
            instances,
            circle,
        }
    }

    pub fn update_points(&mut self, device: &wgpu::Device, camera: &camera::Camera) {
        let circle = Circle::new(self.circle.radius * camera.eye.z, self.circle.segments);

        self.vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&circle.vertices),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
        
        self.index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&circle.indices),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

    }
}
