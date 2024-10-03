use std::sync::Arc;

use winit::{event::*, window::Window};
use wgpu::{self, util::DeviceExt};
use pollster::{block_on, FutureExt};

mod geometry;
mod camera;
mod pipeline;
mod text;

pub use geometry::Color;

pub enum EquationType {
    Polynomial,
    Exponential, // TODO
    Trigonometric, // TODO
    Circle, // TODO
}

pub struct State {
    /*
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: Arc<Window>,
    */
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: camera::CameraController,
    grid_text: text::GridText,
    grid_pipeline: pipeline::GridPipeline,
    point_pipeline: pipeline::PointPipeline,
    equation_pipeline: pipeline::EquationPipeline,
}

impl State {
    pub fn new(device: &wgpu::Device, queue: &wgpu::Queue, config: &wgpu::SurfaceConfiguration) -> State {
        /*
        let window_arc = Arc::new(window);
        let size = window_arc.inner_size();
        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window_arc.clone()).unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        ).block_on().unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ).block_on().unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps.formats.iter()
            .copied()
            .find(|f| f.is_srgb())
            .unwrap_or(surface_caps.formats[0]);
        
        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        */
        let camera = camera::Camera {
            eye: (0.0, 0.0, 4.0).into(),
            target: (0.0, 0.0, 0.0).into(),
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 100.0,
        };

        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera);
        
        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries:  &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("Bind Group Layout"),
        });

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("Camera Bind Group"),
        });

        let camera_controller = camera::CameraController::new(0.1);

        let render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let color_render_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &bind_group_layout,
                &bind_group_layout,
            ],
            push_constant_ranges: &[],
        });

        let size = winit::dpi::PhysicalSize::new(config.width, config.height);

        let point_pipeline = pipeline::PointPipeline::new(device, &render_pipeline_layout, config.format);
        let grid_pipeline = pipeline::GridPipeline::new(device, &render_pipeline_layout, config.format);
        let grid_text = text::GridText::new(device, queue, config.format, size);

        let equation_pipeline = pipeline::EquationPipeline::new(
            device,
            &color_render_pipeline_layout,
            bind_group_layout,
            config.format
        );


        Self {
            /*
            surface,
            device,
            queue,
            config,
            size,
            window: window_arc,
            */
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            grid_text,
            grid_pipeline,
            point_pipeline,
            equation_pipeline,
        }
    }

    /*

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }
    */
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        /*
        self.size = new_size;
        self.config.width = new_size.width;
        self.config.height = new_size.height;
        self.surface.configure(&self.device, &self.config);
        */
        self.grid_text.resize(new_size);

        let new_aspect = new_size.width as f32 / new_size.height as f32;
        if new_aspect <= 3.0 {
            self.camera.aspect = new_aspect;
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    pub fn update(&mut self, queue: &wgpu::Queue, size: winit::dpi::PhysicalSize<u32>) {
        self.camera_controller.update_camera(&mut self.camera, size);
        self.camera_uniform.update_view_proj(&self.camera);
        queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
        self.grid_pipeline.update_grid(queue, &self.camera);
        self.point_pipeline.update_points(queue, &self.camera);
        self.grid_text.viewport.update(queue, glyphon::Resolution { width: size.width, height: size.height });
        self.equation_pipeline.update_equations(queue, &self.camera);
    }

    pub fn grid_prepare(
        &mut self,
        device: &wgpu::Device, 
        queue: &wgpu::Queue, 
        size: winit::dpi::PhysicalSize<u32>
    ) {
        self.grid_text.prepare(
            device, 
            queue,
            size, 
            &self.camera, 
            &self.grid_pipeline.vertical_instances,
            &self.grid_pipeline.horizontal_instances,
        );
    }
    
    pub fn render<'render_pass>(
        &'render_pass self,
        render_pass: &mut wgpu::RenderPass<'render_pass>,
    ) -> Result<(), wgpu::SurfaceError> {

        render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
        // grid rendering
        render_pass.set_pipeline(&self.grid_pipeline.render_pipeline);
        render_pass.set_vertex_buffer(0, self.grid_pipeline.vertical_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.grid_pipeline.vertical_instance_buffer.slice(..));
        render_pass.draw(0..2, 0..self.grid_pipeline.vertical_instances.len() as _);
        render_pass.set_vertex_buffer(0, self.grid_pipeline.horizontal_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.grid_pipeline.horizontal_instance_buffer.slice(..));
        render_pass.draw(0..2, 0..self.grid_pipeline.horizontal_instances.len() as _);

        // equation rendering 
        render_pass.set_pipeline(&self.equation_pipeline.render_pipeline);
        for line in &self.equation_pipeline.lines {
            render_pass.set_bind_group(1, &line.color_bind_group, &[]);
            render_pass.set_vertex_buffer(0, line.vertex_buffer.slice(..));
            render_pass.set_index_buffer(line.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
            render_pass.draw_indexed(0..line.indices.len() as u32, 0, 0..1);
        }
        // point rendering
        render_pass.set_pipeline(&self.point_pipeline.render_pipeline);
        render_pass.set_vertex_buffer(0, self.point_pipeline.vertex_buffer.slice(..));
        render_pass.set_vertex_buffer(1, self.point_pipeline.instance_buffer.slice(..));
        render_pass.set_index_buffer(self.point_pipeline.index_buffer.slice(..), wgpu::IndexFormat::Uint16);
        render_pass.draw_indexed(0..self.point_pipeline.num_indices, 0, 0..self.point_pipeline.instances.len() as _);
        
        self.grid_text.text_renderer.render(&self.grid_text.atlas, &self.grid_text.viewport, render_pass).unwrap(); 

        Ok(())
    }

    pub fn trim_atlas(&mut self) {
        self.grid_text.atlas.trim();
    }
    
    pub fn add_line(&mut self, device: &wgpu::Device, label: u32, coeffs: Vec<f32>, color: geometry::Color<f32>) -> bool {
        self.equation_pipeline.add_line(device, label, coeffs, color)
    }

    pub fn update_line(&mut self, label: u32, coeffs: Vec<f32>) -> bool {
        self.equation_pipeline.update_line(label, coeffs)
    }

    pub fn add_point(&mut self, queue: &wgpu::Queue, point: geometry::Vertex) -> bool {
        self.point_pipeline.add_point(queue, point)
    }
}

