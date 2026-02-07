use diorama_core::math::Vec2;
use diorama_core::triangulate::triangulate;

use crate::vertex::Vertex;

/// Extrude a 2D polygon footprint into a 3D building.
/// `footprint` should be in CCW winding order (top-down XZ plane).
/// Y-axis is up. Building sits on y=0 and rises to y=height.
pub fn extrude_building(
    footprint: &[Vec2],
    height: f32,
    color: [f32; 3],
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let n = footprint.len();
    if n < 3 {
        return (vertices, indices);
    }

    // --- Top face ---
    let top_normal = [0.0, 1.0, 0.0];
    let top_start = vertices.len() as u32;
    for p in footprint {
        vertices.push(Vertex {
            position: [p.x, height, p.y],
            normal: top_normal,
            color,
        });
    }
    let top_tris = triangulate(footprint);
    for idx in &top_tris {
        indices.push(top_start + idx);
    }

    // --- Bottom face ---
    let bottom_normal = [0.0, -1.0, 0.0];
    let bottom_start = vertices.len() as u32;
    for p in footprint {
        vertices.push(Vertex {
            position: [p.x, 0.0, p.y],
            normal: bottom_normal,
            color,
        });
    }
    // Reverse winding for bottom
    for tri in top_tris.chunks(3) {
        indices.push(bottom_start + tri[0]);
        indices.push(bottom_start + tri[2]);
        indices.push(bottom_start + tri[1]);
    }

    // --- Side walls ---
    for i in 0..n {
        let j = (i + 1) % n;
        let a = footprint[i];
        let b = footprint[j];

        // Wall normal (outward, in XZ plane)
        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        let normal = if len > 1e-6 {
            [dy / len, 0.0, -dx / len]
        } else {
            [0.0, 0.0, 1.0]
        };

        let base = vertices.len() as u32;
        // Four corners of the wall quad
        vertices.push(Vertex { position: [a.x, 0.0, a.y], normal, color });
        vertices.push(Vertex { position: [b.x, 0.0, b.y], normal, color });
        vertices.push(Vertex { position: [b.x, height, b.y], normal, color });
        vertices.push(Vertex { position: [a.x, height, a.y], normal, color });

        // Two triangles (CCW when viewed from outside)
        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    (vertices, indices)
}
