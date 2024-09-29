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
            2,
            false,
        );

        Self {
            egui_state,
            egui_renderer,
        }
    }

    pub fn draw(
        &mut self,
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        window: &winit::window::Window,
        render_pass: &mut wgpu::RenderPass<'static>,
        //view: &wgpu::TextureView,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
    ) {
        self.egui_state.egui_ctx().set_pixels_per_point(screen_descriptor.pixels_per_point);

        let input = self.egui_state.take_egui_input(window);
        let full_output = self.egui_state.egui_ctx().run(input, |ctx| {
            egui::CentralPanel::default().show(ctx, |ui| {
                ui.label("test!");
            });
        });

        self.egui_state.handle_platform_output(window, full_output.platform_output);

        let triangles = self.egui_state.egui_ctx().tessellate(
            full_output.shapes, 
            self.egui_state.egui_ctx().pixels_per_point(),
        );
        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer.update_texture(device, queue, *id, image_delta);
        }
        self.egui_renderer.update_buffers(device, queue, encoder, &triangles, screen_descriptor);
        self.egui_renderer.render(render_pass, &triangles, screen_descriptor);

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        // TODO: there should only be one render pass! i haven't decided if this or the engine
        // should own it (or maybe the new state im gonna make loolllll)
        /*
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui render pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view,
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
            self.egui_renderer.render(&mut render_pass, &triangles, screen_descriptor)
        }
        */


       
    }


}
