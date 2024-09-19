use wgpu::{self, include_wgsl, util::DeviceExt};
use cgmath::prelude::*;

use crate::vertex::{Color, Instance, InstanceRaw, Vertex};
use crate::camera;

pub struct Circle {
    pub radius: f32,
    pub segments: u32,
    pub vertices: Vec<Vertex>,
    pub indices: Vec<[u32; 3]>,
}

impl Circle {
    pub fn new(radius: f32, segments: u32) -> Self {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut indices: Vec<[u32; 3]> = Vec::new();

        vertices.push(Vertex { position: [0.0, 0.0, 0.0] });
        indices.push([1, segments, 0]);

        for s in 0..segments {
            // trace the circle and place points along it
            let current_seg = (2.0 * std::f32::consts::PI) * (s as f32 / segments as f32);

            let x = radius * current_seg.cos();
            let y = radius * current_seg.sin();
            let z = 0.0;

            vertices.push(Vertex { position: [x, y, z] });
        }

        for i in 1..segments {
            indices.push([i + 1, i, 0]);
        }

        Self {
            radius,
            segments,
            vertices,
            indices,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use approx::relative_eq;

    #[test]
    fn are_circle_vertices_on_circle() {
        let radius = 1.0;
        let circle = Circle::new(radius, 32);

        for vertex in circle.vertices {
            if vertex.position != [0.0, 0.0, 0.0] {
                let x_squared = vertex.position[0].powf(2.0);
                let y_squared = vertex.position[1].powf(2.0);
                let r_squared = radius.powf(2.0);
                
                assert!(relative_eq!(x_squared + y_squared, r_squared));
            }
        }
    }
}
