use wgpu::{self, include_wgsl, util::DeviceExt};
use cgmath::prelude::*;

use crate::vertex::{Vertex, Instance, InstanceRaw};

pub struct Grid {
    pub render_pipeline: wgpu::RenderPipeline,
    pub horizontal_buffer: wgpu::Buffer,
    pub vertical_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<Instance>,
}

const LINE_HORIZONTAL: &[Vertex] = &[
    Vertex { position: [-100.0, 0.0, 0.0] },
    Vertex { position: [100.0, 0.0, 0.0] },
];

const LINE_VERTICAL: &[Vertex] = &[
    Vertex { position: [0.0, 100.0, 0.0] },
    Vertex { position: [0.0, -100.0, 0.0] },
];

fn get_instances() -> Vec<Instance> {
    let mut instances: Vec<Instance> = vec![];
    let sf = 10.0; // TODO integrate this with zoom scale factor

    for i in -20..20 {
        let position = cgmath::Vector3 { x: i as f32 / sf, y: i as f32 / sf, z: 0.0 };
        let rotation = if position.is_zero() {
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
        };
        
        let a = match i {
            0 => 1.0,
            x if x % 5 == 0 => 0.7,
            _ => 0.4,
        };

        let color = [0.0, 0.0, 0.0, a];

        instances.push(Instance {
            position,
            rotation,
            color,
        });
    }
    instances
}

impl Grid {
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, config: &wgpu::SurfaceConfiguration) -> Self {
        let line_shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Horizontal Grid Render Pipeline"),
            layout: Some(pipeline_layout),
            vertex: wgpu::VertexState {
                module: &line_shader,
                entry_point: "vs_main",
                buffers: &[Vertex::desc(), InstanceRaw::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &line_shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::LineList,
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

        let horizontal_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Horizontal Grid Buffer"),
                contents: bytemuck::cast_slice(&LINE_HORIZONTAL),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        let vertical_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Horizontal Grid Buffer"),
                contents: bytemuck::cast_slice(&LINE_VERTICAL),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );


        let instances = get_instances();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Grid Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );

        Self {
            render_pipeline,
            horizontal_buffer,
            vertical_buffer,
            instance_buffer,
            instances,
        }
    }
}
