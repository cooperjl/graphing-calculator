use wgpu::{self, include_wgsl};
use cgmath::prelude::*;

use crate::vertex::{Vertex, Color, Instance, InstanceRaw};
use crate::camera;

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

pub struct Text {
    pub font_system: glyphon::FontSystem,
    pub swash_cache: glyphon::SwashCache,
    pub viewport: glyphon::Viewport,
    pub atlas: glyphon::TextAtlas,
    pub text_renderer: glyphon::TextRenderer,
    pub x_text_buffer: glyphon::Buffer,
    pub y_text_buffer: glyphon::Buffer,
    pub text_size: f32,
    pub spacing: f32,
}

impl Text {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, format: wgpu::TextureFormat, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let mut font_system = glyphon::FontSystem::new();
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(device);
        let viewport = glyphon::Viewport::new(device, &cache);

        let mut atlas = glyphon::TextAtlas::new(device, queue, &cache, format);
        let text_renderer = glyphon::TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        let text_size = 21.0;
        let spacing = text_size;
        let mut x_text_buffer = glyphon::Buffer::new(&mut font_system, glyphon::Metrics::new(text_size, spacing));
        let mut y_text_buffer = glyphon::Buffer::new(&mut font_system, glyphon::Metrics::new(text_size, spacing));

        let physical_width = size.width as f32 * 2.0;
        let physical_height = size.height as f32 * 2.0;

        x_text_buffer.set_size(
            &mut font_system,
            Some(physical_width),
            Some(physical_height),
        );

        y_text_buffer.set_size(
            &mut font_system,
            Some(physical_width),
            Some(physical_height),
        );

        x_text_buffer.shape_until_scroll(&mut font_system, false);
        y_text_buffer.shape_until_scroll(&mut font_system, false);

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            x_text_buffer,
            y_text_buffer,
            text_size,
            spacing,
        }
    }
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: winit::dpi::PhysicalSize<u32>, camera: &camera::Camera, vertical_instances: &Vec<Instance>, horizontal_instances: &Vec<Instance>) {
        let mut y_text: String = "".to_owned();
        for instance in horizontal_instances {
            let num = instance.position.y;
            if instance.color.a == 0.7 {
                y_text.push_str(format!("{num}").as_str());
            } 
            y_text.push('\n');
        }
        let mut x_text: String = "".to_owned();
        for instance in vertical_instances {
            let num = instance.position.x;
            if instance.color.a == 0.7 {
                x_text.push_str(format!("{num}").as_str());
            } 
            x_text.push('\n');
        }

        let attrs = glyphon::Attrs::new()
            .family(glyphon::Family::Monospace);

        self.x_text_buffer.set_text(&mut self.font_system, x_text.as_str(), attrs, glyphon::Shaping::Advanced);
        self.y_text_buffer.set_text(&mut self.font_system, y_text.as_str(), attrs, glyphon::Shaping::Advanced);

        let axis_pos = camera.world_to_screen_space(-camera.eye.to_vec(), size);
        let position_offset = self.text_size / 2.0;

        let mut text_areas: Vec<glyphon::TextArea> = vec![];
        for (i, instance) in vertical_instances.iter().enumerate() {
            let text_pos = camera.world_to_screen_space(instance.position, size);

            let bound_offset = i as f32 * self.spacing;

            let text_area = glyphon::TextArea {
                buffer: &self.x_text_buffer,
                left: if instance.position.x == 0.0 { axis_pos.x } else { text_pos.x - position_offset },
                top:  axis_pos.y - bound_offset,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: (text_pos.x - position_offset) as i32,
                    top: axis_pos.y as i32,
                    right: size.width as i32,
                    bottom: (axis_pos.y + self.text_size) as i32,
                },
                default_color: glyphon::Color::rgb(0, 0, 0),
            };
            text_areas.push(text_area);
        }
        for (i, instance) in horizontal_instances.iter().enumerate() {
            let text_pos = camera.world_to_screen_space(instance.position, size);

            let bound_offset = i as f32 * self.spacing;

            let text_area = glyphon::TextArea {
                buffer: &self.y_text_buffer,
                left: axis_pos.x,
                top: text_pos.y - bound_offset - position_offset,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: axis_pos.x as i32,
                    top: (text_pos.y - position_offset) as i32,
                    right: size.width as i32,
                    bottom: (text_pos.y + self.text_size - position_offset) as i32,
                },
                default_color: glyphon::Color::rgb(0, 0, 0),
            };
            text_areas.push(text_area);

            // avoid doubling up the origin label
            // origin label disabled so code disabled, remove above text_areas.push if using
            /*
            if instance.position.y != 0.0 {
                text_areas.push(text_area);
            }
            */
        }

        self.text_renderer.prepare(
            device,
            queue,
            &mut self.font_system,
            &mut self.atlas,
            &self.viewport,
            text_areas,
            &mut self.swash_cache,
        )
        .unwrap();
    }

    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        let physical_width = new_size.width as f32 * 4.0;
        let physical_height = new_size.height as f32 * 4.0;

        self.x_text_buffer.set_size(
            &mut self.font_system,
            Some(physical_width),
            Some(physical_width),
        );
        self.y_text_buffer.set_size(
            &mut self.font_system,
            Some(physical_height),
            Some(physical_height),
        );
    }
}

pub struct Grid {
    pub render_pipeline: wgpu::RenderPipeline,
    pub horizontal_buffer: wgpu::Buffer,
    pub vertical_buffer: wgpu::Buffer,
    pub vertical_instance_buffer: wgpu::Buffer,
    pub horizontal_instance_buffer: wgpu::Buffer,
    pub vertical_instances: Vec<Instance>,
    pub horizontal_instances: Vec<Instance>,
}

impl Grid {
    pub fn new(device: &wgpu::Device, pipeline_layout: &wgpu::PipelineLayout, config: &wgpu::SurfaceConfiguration) -> Self {
        let line_shader = device.create_shader_module(include_wgsl!("shader.wgsl"));

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Grid Render Pipeline"),
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vertical() {
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
