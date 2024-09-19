use winit::{
    event::*, event_loop::EventLoop, window::{Window, WindowBuilder}
};

use wgpu::{self, util::DeviceExt};

mod vertex;
mod camera;
mod grid;
mod points;
mod eqn;
mod pipeline;

struct State<'a> {
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    window: &'a Window,
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: camera::CameraController,
    grid: grid::Grid,
    grid_text: grid::Text,
    point_pipeline: pipeline::PointPipeline,
    equation_pipeline: pipeline::EquationPipeline,
}

impl<'a> State<'a> {
    async fn new(window: &'a Window) -> State<'a> {
        let size = window.inner_size();
        let instance = wgpu::Instance::default();

        let surface = instance.create_surface(window).unwrap();

        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                force_fallback_adapter: false,
                compatible_surface: Some(&surface),
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                label: None,
                required_features: wgpu::Features::empty(),
                required_limits: wgpu::Limits::default(),
                memory_hints: wgpu::MemoryHints::Performance,
            },
            None,
        ).await.unwrap();

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

        let point_pipeline = pipeline::PointPipeline::new(&device, &render_pipeline_layout, config.format);
        let grid = grid::Grid::new(&device, &render_pipeline_layout, &config);
        let grid_text = grid::Text::new(&device, &queue, surface_format, size);

        let mut equation_pipeline = pipeline::EquationPipeline::new(
            &device,
            &color_render_pipeline_layout,
            &bind_group_layout,
            config.format
        );
        equation_pipeline.update_equations(&queue, &camera);
        // point_pipeline.put_points(&queue, &equation_pipeline.lines[0].vertices);

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window,
            camera,
            camera_uniform,
            camera_buffer,
            camera_bind_group,
            camera_controller,
            grid,
            grid_text,
            point_pipeline,
            equation_pipeline,
        }
    }

    pub fn window(&self) -> &Window {
        self.window
    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.grid_text.resize(new_size);

            let new_aspect = new_size.width as f32 / new_size.height as f32;
            if new_aspect <= 3.0 {
                self.camera.aspect = new_aspect;
            }
        }
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.redraw();
        self.camera_controller.process_events(event)

    }

    fn update(&mut self) {
        self.camera_controller.update_camera(&mut self.camera, self.size);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
        self.grid.update_grid(&self.queue, &self.camera);
        self.point_pipeline.update_points(&self.queue, &self.camera);
        self.grid_text.viewport.update(&self.queue, glyphon::Resolution { width: self.config.width, height: self.config.height });
    }

    fn redraw(&mut self) {
        self.equation_pipeline.update_equations(&self.queue, &self.camera);
    }

    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.grid_text.prepare(&self.device, &self.queue, self.size, &self.camera, &self.grid.vertical_instances, &self.grid.horizontal_instances);

        let output = self.surface.get_current_texture()?;

        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);
            // grid rendering
            render_pass.set_pipeline(&self.grid.render_pipeline);
            render_pass.set_vertex_buffer(0, self.grid.vertical_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.grid.vertical_instance_buffer.slice(..));
            render_pass.draw(0..2, 0..self.grid.vertical_instances.len() as _);
            render_pass.set_vertex_buffer(0, self.grid.horizontal_buffer.slice(..));
            render_pass.set_vertex_buffer(1, self.grid.horizontal_instance_buffer.slice(..));
            render_pass.draw(0..2, 0..self.grid.horizontal_instances.len() as _);
            // equation rendering
            render_pass.set_pipeline(&self.equation_pipeline.render_pipeline);
            //render_pass.set_vertex_buffer(1, self.equations.instance_buffer.slice(..));
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

            self.grid_text.text_renderer.render(&self.grid_text.atlas, &self.grid_text.viewport, &mut render_pass).unwrap();
        }
        
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.grid_text.atlas.trim();

        Ok(())
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let window = WindowBuilder::new().build(&event_loop).unwrap();
    let mut state = State::new(&window).await;

    event_loop.run(move |event, elwt| {
        match event {
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == state.window().id() => if !state.input(event) {
                match &event {
                    WindowEvent::Resized(physical_size) => state.resize(*physical_size),
                    WindowEvent::CloseRequested => elwt.exit(),
                    WindowEvent::RedrawRequested => {
                        state.window().request_redraw();
                        state.update();

                        match state.render() {
                            Ok(_) => {}
                            Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                            Err(wgpu::SurfaceError::OutOfMemory) => elwt.exit(),
                            Err(e) => eprintln!("{:?}", e),
                        }
                    },
                    _ => {}
                }
            }
            _ => {}
        }
    }).unwrap();
}
