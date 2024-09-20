use cgmath::prelude::*;

use crate::camera;
use crate::vertex::Instance;

pub struct GridText {
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

impl GridText {
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
                custom_glyphs: &[],
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
                custom_glyphs: &[],
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
