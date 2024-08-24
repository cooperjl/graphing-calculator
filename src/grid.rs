use wgpu::{self, include_wgsl};
use cgmath::prelude::*;

use crate::vertex::{Vertex, Instance, InstanceRaw};
use crate::camera;

fn get_instances(camera: &camera::Camera) -> Vec<Instance> {
    let base_spacing = 20.0;
    let sf = base_spacing / (camera.eye.z as u32).next_power_of_two() as f32;

    let mut instances: Vec<Instance> = vec![];

    let bound = (base_spacing * 1.5) as i32;

    for i in -bound..bound {
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

pub struct Text {
    pub font_system: glyphon::FontSystem,
    pub swash_cache: glyphon::SwashCache,
    pub viewport: glyphon::Viewport,
    pub atlas: glyphon::TextAtlas,
    pub text_renderer: glyphon::TextRenderer,
    pub text_buffer: glyphon::Buffer,
    pub text_size: f32,
    pub spacing: f32,
}

impl Text {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, size: winit::dpi::PhysicalSize<u32>) -> Self {
        let swapchain_format = wgpu::TextureFormat::Rgba8UnormSrgb;
        let mut font_system = glyphon::FontSystem::new();
        let swash_cache = glyphon::SwashCache::new();
        let cache = glyphon::Cache::new(&device);
        let viewport = glyphon::Viewport::new(&device, &cache);

        let mut atlas = glyphon::TextAtlas::new(device, queue, &cache, swapchain_format);
        let text_renderer = glyphon::TextRenderer::new(&mut atlas, device, wgpu::MultisampleState::default(), None);
        let text_size = 21.0;
        let spacing = text_size;
        let mut text_buffer = glyphon::Buffer::new(&mut font_system, glyphon::Metrics::new(text_size, spacing));

        let physical_width = size.width as f32 * 2.0;
        let physical_height = size.height as f32 * 2.0;

        text_buffer.set_size(
            &mut font_system,
            Some(physical_width),
            Some(physical_height),
        );

        text_buffer.shape_until_scroll(&mut font_system, false);

        Self {
            font_system,
            swash_cache,
            viewport,
            atlas,
            text_renderer,
            text_buffer,
            text_size,
            spacing,
        }
    }
    
    pub fn prepare(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, size: winit::dpi::PhysicalSize<u32>, instances: &Vec<Instance>, camera: &camera::Camera) {
        let mut text: String = "".to_owned();
        for (i, instance) in instances.iter().enumerate() {
            let num = instance.position.y as f32;
            if i % 5 == 0 {
                text.push_str(format!("{num}").as_str());
            } 
            text.push_str("\n");
        }

        let attrs = glyphon::Attrs::new()
            .family(glyphon::Family::Monospace);

        self.text_buffer.set_text(&mut self.font_system, text.as_str(), attrs, glyphon::Shaping::Advanced);

        let mut text_areas: Vec<glyphon::TextArea> = vec![];
        for (i, instance) in instances.iter().enumerate() {
            let center_width = size.width as f32 / 2.0;
            let center_height = size.height as f32 / 2.0;

            let cgmath::Vector2 { x, y } = camera.world_to_screen_space(instance.position, size);

            let offset = i as f32 * self.spacing;

            // x axis
            let text_area = glyphon::TextArea {
                buffer: &self.text_buffer,
                left: x,
                top:  center_height - offset,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: x as i32,
                    top: center_height as i32,
                    right: size.width as i32,
                    bottom: (center_height + self.text_size) as i32,
                },
                default_color: glyphon::Color::rgb(0, 0, 0),
            };
            text_areas.push(text_area);

            // y axis
            let text_area = glyphon::TextArea {
                buffer: &self.text_buffer,
                left: center_width,
                top: y - offset,
                scale: 1.0,
                bounds: glyphon::TextBounds {
                    left: center_width as i32,
                    top: y as i32,
                    right: size.width as i32,
                    bottom: (y + self.text_size) as i32,
                },
                default_color: glyphon::Color::rgb(0, 0, 0),
            };
            
            // avoid doubling up the origin label
            if instance.position.y != 0.0 {
                text_areas.push(text_area);
            }
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
        let physical_width = new_size.width as f32 * 2.0;
        let physical_height = new_size.height as f32 * 2.0;

        self.text_buffer.set_size(
            &mut self.font_system,
            Some(physical_width),
            Some(physical_height),
        );
    }
}

pub struct Grid {
    pub render_pipeline: wgpu::RenderPipeline,
    pub horizontal_buffer: wgpu::Buffer,
    pub vertical_buffer: wgpu::Buffer,
    pub instance_buffer: wgpu::Buffer,
    pub instances: Vec<Instance>,
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
        let instance_buffer = device.create_buffer(
            &wgpu::BufferDescriptor {
                label: Some("Grid Instance Buffer"),
                size: 6400,
                usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            }
        );

        let instances = vec![];

        Self {
            render_pipeline,
            horizontal_buffer,
            vertical_buffer,
            instance_buffer,
            instances,
        }
    }
    
    pub fn update_grid(&mut self, queue: &wgpu::Queue, camera: &camera::Camera) {
        self.instances = get_instances(camera);
        self.set_buffers(queue, camera.eye.z);
    }

    fn set_buffers(&self, queue: &wgpu::Queue, sf: f32) {
        let line_limit = sf * 1.5;

        let line_horizontal: &[Vertex] = &[
            Vertex { position: [-line_limit, 0.0, 0.0] },
            Vertex { position: [line_limit, 0.0, 0.0] },
        ];

        let line_vertical: &[Vertex] = &[
            Vertex { position: [0.0, line_limit, 0.0] },
            Vertex { position: [0.0, -line_limit, 0.0] },
        ];

        let instance_data = self.instances.iter().map(Instance::to_raw).collect::<Vec<_>>();

        queue.write_buffer(&self.horizontal_buffer, 0, bytemuck::cast_slice(&line_horizontal));
        queue.write_buffer(&self.vertical_buffer, 0, bytemuck::cast_slice(&line_vertical));
        queue.write_buffer(&self.instance_buffer, 0, bytemuck::cast_slice(&instance_data));
    }
}
