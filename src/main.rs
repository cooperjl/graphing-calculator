mod graphing_engine;
mod gui;

use std::sync::Arc;

use pollster::{block_on, FutureExt};
use winit::application::ApplicationHandler;
use winit::event::WindowEvent;
use winit::event_loop::{ActiveEventLoop, EventLoop};
use winit::window::{Window, WindowId};
use winit::dpi::PhysicalSize;
use rand::Rng;

use graphing_engine::State;
use graphing_engine::Color;

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();

    let mut window_state = App::new();
    let _ = event_loop.run_app(&mut window_state);
}

struct App {
    state: Option<AppState>,
}

impl App {
    pub fn new() -> Self {
        Self { 
            state: None,
        }
    }
}

impl ApplicationHandler for App {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("graphing calculator"))
            .unwrap();
        self.state = Some(AppState::new(window));
    }

    fn window_event(
            &mut self,
            event_loop: &ActiveEventLoop,
            window_id: WindowId,
            event: WindowEvent,
        ) {
        let state = self.state.as_mut().unwrap();

        if window_id == state.window().id() && !state.input(&event) {
            match event {
                WindowEvent::Resized(physical_size) => state.resize(physical_size),
                WindowEvent::CloseRequested => event_loop.exit(),
                WindowEvent::RedrawRequested => {
                    state.graphing_engine.update(&state.queue, state.size());

                    match state.render() {
                        Ok(_) => {}
                        Err(wgpu::SurfaceError::Lost) => state.resize(state.size()),
                        Err(wgpu::SurfaceError::OutOfMemory) => event_loop.exit(),
                        Err(e) => eprintln!("{:?}", e),
                    }
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, _event_loop: &ActiveEventLoop) {
        let window = self.state.as_ref().unwrap().window();
        window.request_redraw();
    }
}

struct AppState {
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,

    size: PhysicalSize<u32>,
    window: Arc<Window>,

    graphing_engine: graphing_engine::State,
    gui_renderer: gui::GuiRenderer,

    equations: Vec<String>,

}

impl AppState {
    pub fn new(window: Window) -> Self {
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

        let graphing_engine = State::new(&device, &queue, &config);
        
        let gui_renderer = gui::GuiRenderer::new(&device, &window_arc, config.format);

        let equations = Vec::new();

        Self {
            surface,
            device,
            queue,
            config,
            size,
            window: window_arc,
            graphing_engine,
            gui_renderer,
            equations,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }

    pub fn size(&self) -> winit::dpi::PhysicalSize<u32> {
        self.size
    }
    pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;

            self.config.width = new_size.width;
            self.config.height = new_size.height;

            self.surface.configure(&self.device, &self.config);

            self.graphing_engine.resize(new_size);
        }
    }

    pub fn input(&mut self, event: &WindowEvent) -> bool {
        self.gui_renderer.input(&self.window, event) || self.graphing_engine.input(event)
    }

    pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
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

            self.graphing_engine.grid_prepare(&self.device, &self.queue, self.size);
            
            match self.graphing_engine.render(&mut render_pass) {
                Ok(_) => {}
                Err(e) => eprintln!("{:?}", e),
            }
        }

        {
            let screen_descriptor = egui_wgpu::ScreenDescriptor {
                size_in_pixels: [self.config.width, self.config.height],
                pixels_per_point: self.window().scale_factor() as f32 * 1.0,
            };

            self.gui_renderer.begin_pass(&self.window);

            egui::SidePanel::new(
                egui::panel::Side::Left, 
                egui::Id::new("left panel")
                )
                .show(self.gui_renderer.ctx(), |ui| {
                    ui.label("Equations");
                    if ui.button("+").clicked() {
                        self.equations.push(String::new());
                        let r = rand::thread_rng().gen_range(0.0..=1.0);
                        let g = rand::thread_rng().gen_range(0.0..=1.0);
                        let b = rand::thread_rng().gen_range(0.0..=1.0);
                        let color = Color { r, g, b, a: 1.0 };

                        self.graphing_engine.add_line(&self.device, self.equations.len() as u16 - 1, Vec::new(), color);
                    }
                    for (i, equation) in self.equations.iter_mut().enumerate() {
                        let response = ui.text_edit_singleline(equation);

                        if response.changed() {
                            self.graphing_engine.update_line(i as u16, equation);
                        }
                    }
                });

            self.gui_renderer.render(
                &self.device,
                &self.queue,
                &mut encoder,
                &self.window,
                &view,
                &screen_descriptor,
            );
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
        self.graphing_engine.trim_atlas();
        
        Ok(())
    }
}

fn main() {
    block_on(run());
}
