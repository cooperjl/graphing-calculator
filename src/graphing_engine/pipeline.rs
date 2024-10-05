use wgpu::{self, util::DeviceExt, include_wgsl};
use cgmath::prelude::*;
use regex::Regex;
use anyhow::Result;

use crate::graphing_engine::camera;
use crate::graphing_engine::geometry::{Vertex, Instance, InstanceRaw, Color, Circle, Line};

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

fn get_instances(camera: &camera::Camera, vertical: bool) -> Vec<Instance> {
    let base_spacing = 40.0;
    let sf = base_spacing / (camera.eye.z as u32).next_power_of_two() as f32;

    let mut instances: Vec<Instance> = Vec::new();

    let offset = if vertical {
        camera.eye.x * sf
    } else {
        camera.eye.y * sf
    } as i32;
    
    let bound_l = (base_spacing * -2.0) as i32 + offset;
    let bound_r = (base_spacing * 2.0) as i32 + offset;

    for i in bound_l..bound_r {
        let x = if vertical {
            i as f32 / sf
        } else {
            camera.eye.x
        };
        let y = if !vertical {
            i as f32 / sf
        } else {
            camera.eye.y
        };
        let position = cgmath::Vector3 { x, y, z: 0.0 };
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

        let color = Color { r: 0.0, g: 0.0, b: 0.0, a };

        instances.push(Instance {
            position,
            rotation,
            color,
        });
    }
    instances
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
    color_bind_group_layout: wgpu::BindGroupLayout,
}

impl EquationPipeline {
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, color_bind_group_layout: wgpu::BindGroupLayout, format: wgpu::TextureFormat) -> Self {
        let render_pipeline = create_render_pipeline(
            device, 
            pipeline_layout, 
            format, 
            &[Vertex::desc(), InstanceRaw::desc()],
            include_wgsl!("eqn_shader.wgsl"),
            wgpu::PrimitiveTopology::TriangleList,
        );
        
        let lines = Vec::new();

        Self {
            render_pipeline,
            lines,
            color_bind_group_layout,
        }
    }

    pub fn add_line(&mut self, device: &wgpu::Device, label: u32, coeffs: Vec<f32>, color: Color<f32>) -> bool {
        // TODO: use dict with label
        let line = Line::new(device, coeffs, 0.025, color, &self.color_bind_group_layout);
        self.lines.push(line);
        true
    }

    pub fn update_line(&mut self, label: u32, equation: &str) -> bool {
        match self.parse_equation(equation) {
            Ok(coeffs) => {
                match self.lines.get_mut(label as usize) {
                    Some(line) => {
                        line.coeffs = coeffs;
                        true
                    }
                    None => false
                }

            }
            Err(_) => false, // TODO: remove line in this case since it is broken
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

    /// Returns coefficients for Line::make_polynomial if successful. 
    ///
    /// Takes a string which represents a polynomial equation, using ^ to represent exponent.
    fn parse_equation(&self, equation: &str) -> Result<Vec<f32>> {
        // TODO: possibly expensive so reuse this as explained in regex docs
        let re = Regex::new(r"([+-]?[^+-]+)").unwrap();
        let split_eqn = equation.split_whitespace().collect::<String>();
        
        let mut coeffs: Vec<f32> = Vec::new();

        let eqn: Vec<_> = re.find_iter(split_eqn.as_str()).map(|m| m.as_str()).collect();

        for exp in eqn {
            let parts = exp.split('x').collect::<Vec<_>>();

            let key = if parts.len() > 1 {
                let last = parts.last().unwrap();

                if !last.is_empty() {
                    last[1..].parse::<u32>()?
                } else {
                    1
                }
            } else {
                0
            };
            
            let first = parts.first().unwrap();
            let val = if first.is_empty() {
                1.0
            } else {
                parts.first().unwrap().parse::<f32>()?
            };

            match coeffs.get_mut(key as usize) {
                Some(o) => *o += val,
                None => {
                    coeffs.resize(key as usize, 0.0);
                    coeffs.push(val);
                }
            }
        }

        Ok(coeffs)
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

        let num_indices = (circle.segments * 3).into();

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

    pub fn add_point(&mut self, queue: &wgpu::Queue, point: Vertex) -> bool {
        let position = cgmath::Vector3 { x: point.position[0], y: point.position[1], z: 0.0 };
        let rotation = if position.is_zero() {
            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
        } else {
            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(0.0))
        };
        let color = Color { r: 0.0, g: 0.0, b: 0.0, a: 1.0 };

        self.instances.push(Instance {
            position,
            rotation,
            color,
        });

        let instance_data = &self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(instance_data));

        true
    }
}


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn get_instances_vertical() {
        let x = 5.0;
        let y = 200.0;
        let camera = camera::Camera {
            eye: (x, y, 4.0).into(),
            target: (x, y, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: 1.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let v_instances = get_instances(&camera, true);
        let h_instances = get_instances(&camera, false);

        for (v_instance, h_instance) in v_instances.iter().zip(h_instances.iter()) {
            // they will share a common point in the center
            if v_instance.position.x != x && v_instance.position.y != y {
                // assert the positions are different as they should be here if vertical functions
                assert_ne!(v_instance.position, h_instance.position);
            }
        }
    }

    #[test]
    fn next_zoom_level_is_double() {
        // using a zoom level of 20 for testing purposes
        let zoom_level = 20_u32.next_power_of_two() as f32;
        let camera1 = camera::Camera {
            eye: (0.0, 0.0, zoom_level).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: 1.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };
        let camera2 = camera::Camera {
            eye: (0.0, 0.0, zoom_level * 2.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: 1.0,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        // for vertical / x
        let instances1 = get_instances(&camera1, true);
        let instances2 = get_instances(&camera2, true);

        for (instance1, instance2) in instances1.iter().zip(instances2.iter()) {
            assert_eq!(instance1.position.x * 2.0, instance2.position.x);
        }

        // for horizontal / y
        let instances1 = get_instances(&camera1, false);
        let instances2 = get_instances(&camera2, false);

        for (instance1, instance2) in instances1.iter().zip(instances2.iter()) {
            assert_eq!(instance1.position.y * 2.0, instance2.position.y);
        }
    }
}

