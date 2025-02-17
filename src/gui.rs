pub struct GuiRenderer {
    egui_state: egui_winit::State,
    egui_renderer: egui_wgpu::Renderer,
}

impl GuiRenderer {
    pub fn new(
        device: &wgpu::Device,
        window: &winit::window::Window,
        color_format: wgpu::TextureFormat,
    ) -> Self {
        let egui_context = egui::Context::default();

        let egui_state = egui_winit::State::new(
            egui_context, 
            egui::viewport::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
            None,
        );
        let egui_renderer = egui_wgpu::Renderer::new(
            device, 
            color_format,
            None,
            1,
            false,
        );


        Self {
            egui_state,
            egui_renderer,
        }
    }

    pub fn input(&mut self, window: &winit::window::Window, event: &winit::event::WindowEvent) -> bool {
        self.egui_state.on_window_event(window, event).consumed
    }

    pub fn ctx(&self) -> &egui::Context {
        self.egui_state.egui_ctx()
    }

    pub fn begin_pass(&mut self, window: &winit::window::Window) {
        let input = self.egui_state.take_egui_input(window);
        self.ctx().begin_pass(input);
    }

    pub fn render(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &winit::window::Window,
        view: &wgpu::TextureView,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
    ) {
        self.ctx().set_pixels_per_point(screen_descriptor.pixels_per_point);

        let full_output = self.ctx().end_pass();

        self.egui_state.handle_platform_output(window, full_output.platform_output);

        let triangles = self.ctx().tessellate(
            full_output.shapes, 
            self.ctx().pixels_per_point(),
        );
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(device, queue, *id, image_delta);
        }

        self.egui_renderer.update_buffers(device, queue, encoder, &triangles, screen_descriptor);
        {
            let render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("gui_pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            self.egui_renderer.render(&mut render_pass.forget_lifetime(), &triangles, screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }
        
    }
}
