use wgpu::util::DeviceExt;
use wgpu::{self, include_wgsl};
use cgmath::prelude::*;

use crate::vertex::{Vertex, Instance, InstanceRaw, Color};
use crate::camera;

pub struct Equation {
    pub render_pipeline: wgpu::RenderPipeline,
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub num_indices: u32,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<Instance>,
}

impl Equation {
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, config: &wgpu::SurfaceConfiguration) -> Self {
        let eqn_shader = device.create_shader_module(include_wgsl!("eqn_shader.wgsl"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Render Pipeline"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: &eqn_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &eqn_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
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

        let vertex_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Equation Vertex Buffer"),
                size: 48,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Equation Index Buffer"),
                contents: bytemuck::cast_slice(&[
                    3, 2, 0,
                    2, 1, 0,
                ]),
                usage: wgpu::BufferUsages::INDEX,
            }
        );

        let num_indices = 12;

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

        // temporary concept for now, will likely use something like grmtools to parse
        // equations here and feed them into the shader (somehow)

        Self {
            render_pipeline,
            vertex_buffer,
            index_buffer,
            num_indices,
            instance_buffer,
            instances,
        }
    }

    pub fn update_equations(&self, queue: &wgpu::Queue, camera: &camera::Camera) {
        // TODO read base_spacing from somewhere here and in grid instead of hardcoding
        let base_spacing = 40.0;

        let x = base_spacing + camera.eye.x.abs();
        let y = base_spacing + camera.eye.y.abs();

        let square: &[Vertex] = &[
            Vertex { position: [-x, -y, 0.0] },
            Vertex { position: [ x, -y, 0.0] },
            Vertex { position: [ x,  y, 0.0] },
            Vertex { position: [-x,  y, 0.0] },
        ];

        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(square));
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data));
    }
}
