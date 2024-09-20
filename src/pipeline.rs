use wgpu::{self, util::DeviceExt, include_wgsl};
use cgmath::prelude::*;

use crate::eqn::Line;
use crate::camera;
use crate::vertex::{Color, Instance, InstanceRaw, Vertex};
use crate::points::Circle;
use crate::grid::get_instances;

fn create_render_pipeline(
    device: &wgpu::Device,
    layout: &wgpu::PipelineLayout,
    format: wgpu::TextureFormat,
    vertex_layouts: &[wgpu::VertexBufferLayout],
    shader: wgpu::ShaderModuleDescriptor,
    topology: wgpu::PrimitiveTopology,
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
            topology,
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

pub struct GridPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub horizontal_buffer: wgpu::Buffer,
    pub vertical_buffer: wgpu::Buffer,
    pub vertical_instance_buffer: wgpu::Buffer,
    pub horizontal_instance_buffer: wgpu::Buffer,
    pub vertical_instances: Vec<Instance>,
    pub horizontal_instances: Vec<Instance>,
}

impl GridPipeline {
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, format: wgpu::TextureFormat) -> Self {
        let render_pipeline = create_render_pipeline(
            device, 
            pipeline_layout, 
            format,
            &[Vertex::desc(), InstanceRaw::desc()],
            include_wgsl!("shader.wgsl"),
            wgpu::PrimitiveTopology::LineList,
        );
        
        let horizontal_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Horizontal Grid Buffer"),
                size: 24,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let vertical_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Vertical Grid Buffer"),
                size: 24,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let vertical_instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Grid Instance Buffer"),
                size: 12800,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let horizontal_instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Grid Instance Buffer"),
                size: 12800,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let horizontal_instances = vec![];
        let vertical_instances = vec![];

        Self {
            render_pipeline,
            horizontal_buffer,
            vertical_buffer,
            vertical_instance_buffer,
            horizontal_instance_buffer,
            horizontal_instances,
            vertical_instances,
        }
    }
    
    pub fn update_grid(&mut self, queue: &wgpu::Queue, camera: &camera::Camera) {
        self.vertical_instances = get_instances(camera, true);
        self.horizontal_instances = get_instances(camera, false);
        self.set_buffers(queue, camera.eye.z);
    }

    fn set_buffers(&self, queue: &wgpu::Queue, sf: f32) {
        let line_limit = sf * 2.0;

        let line_horizontal: &[Vertex] = &[
            Vertex { position: [-line_limit, 0.0, 0.0] },
            Vertex { position: [line_limit, 0.0, 0.0] },
        ];

        let line_vertical: &[Vertex] = &[
            Vertex { position: [0.0, line_limit, 0.0] },
            Vertex { position: [0.0, -line_limit, 0.0] },
        ];

        let vertical_instance_data = self.vertical_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        let horizontal_instance_data = self.horizontal_instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        queue.write_buffer(&self.horizontal_buffer, 0, bytemuck::cast_slice(line_horizontal));
        queue.write_buffer(&self.vertical_buffer, 0, bytemuck::cast_slice(line_vertical));
        queue.write_buffer(&self.horizontal_instance_buffer, 0, bytemuck::cast_slice(&horizontal_instance_data));
        queue.write_buffer(&self.vertical_instance_buffer, 0, bytemuck::cast_slice(&vertical_instance_data));
    }
}

pub struct EquationPipeline {
    pub render_pipeline: wgpu::RenderPipeline,
    pub lines: Vec<Line>,
}

impl EquationPipeline {
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, color_bind_group_layout: &wgpu::BindGroupLayout, format: wgpu::TextureFormat) -> Self {
        let render_pipeline = create_render_pipeline(
            device, 
            pipeline_layout, 
            format, 
            &[Vertex::desc(), InstanceRaw::desc()],
            include_wgsl!("eqn_shader.wgsl"),
            wgpu::PrimitiveTopology::TriangleList,
        );

        let color1 = Color { r: 1.0, g: 0.0, b: 0.0, a: 1.0 };
        let color2 = Color { r: 0.0, g: 0.0, b: 1.0, a: 1.0 };

        let coeffs1 = vec![1.0, 0.0];
        let coeffs2 = vec![1.0, 4.0, 3.0, -1.0];

        let line1 = Line::new(device, coeffs1, 0.025, color1, color_bind_group_layout);
        let line2 = Line::new(device, coeffs2, 0.025, color2, color_bind_group_layout);

        let lines = vec![line1, line2];

        Self {
            render_pipeline,
            lines,
        }
    }

    pub fn update_equations(&mut self, queue: &wgpu::Queue, camera: &camera::Camera) {
        let width = 0.004 * camera.eye.z;
        let range = camera.eye.z * 1.5;
        let x_min = -range + camera.eye.x;
        let x_max = range + camera.eye.x;

        for line in &mut self.lines {
            line.width = width;
            line.make_polynomial(x_min as i32, x_max as i32);
            line.update_buffers(queue);
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
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, format: wgpu::TextureFormat) -> Self {
        let render_pipeline = create_render_pipeline(
            device, 
            pipeline_layout, 
            format, 
            &[Vertex::desc(), InstanceRaw::desc()],
            include_wgsl!("shader.wgsl"),
            wgpu::PrimitiveTopology::TriangleList,
        );

        let circle = Circle::new(0.005, 32);

        let vertex_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Vertex Buffer"),
                contents: bytemuck::cast_slice(&circle.vertices),
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            }
        );
        
        let index_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Index Buffer"),
                contents: bytemuck::cast_slice(&circle.indices),
                usage: wgpu::BufferUsages::INDEX | wgpu::BufferUsages::COPY_DST,
            }
        );

        let num_indices = circle.segments * 6;

        let instances: Vec<Instance> = Vec::new();
        
        let instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Points Instance Buffer"),
                size: 100000,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
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

    pub fn update_points(&mut self, queue: &wgpu::Queue, camera: &camera::Camera) {
        let circle = Circle::new(self.circle.radius * camera.eye.z, self.circle.segments);

        queue.write_buffer(&self.vertex_buffer, 0, bytemuck::cast_slice(&circle.vertices));
        queue.write_buffer(&self.index_buffer, 0, bytemuck::cast_slice(&circle.indices));
    }

    pub fn put_points(&mut self, queue: &wgpu::Queue, points: &Vec<Vertex>) {
        for point in points {
            //let position = point.position;
            let position = cgmath::Vector3 { x: point.position[0], y: point.position[1], z: 0.0 };
            let rotation = if position.is_zero() {
                cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
            } else {
                cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
            };
            let color = Color { r: 0.0, g: 1.0, b: 0.0, a: 1.0 };

            self.instances.push(Instance {
                position,
                rotation,
                color,
            });
        }

        let instance_data = &self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instance_data));
    }
}
