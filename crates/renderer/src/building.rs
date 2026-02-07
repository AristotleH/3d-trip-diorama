use diorama_core::math::Vec2;
use diorama_core::triangulate::triangulate;

use crate::vertex::Vertex;

/// Extrude a 2D polygon footprint into a 3D building.
/// `footprint` should be in CCW winding order (top-down XZ plane).
/// Y-axis is up. Building sits on y=base_y and rises to y=base_y+height.
pub fn extrude_building(
    footprint: &[Vec2],
    height: f32,
    base_y: f32,
    color: [f32; 3],
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let n = footprint.len();
    if n < 3 {
        return (vertices, indices);
    }

    let top_y = base_y + height;

    // Slightly darken walls vs roof for visual variety
    let roof_color = [
        (color[0] * 1.05).min(1.0),
        (color[1] * 1.05).min(1.0),
        (color[2] * 1.05).min(1.0),
    ];

    // --- Top face (roof) ---
    let top_normal = [0.0, 1.0, 0.0];
    let top_start = vertices.len() as u32;
    for p in footprint {
        vertices.push(Vertex {
            position: [p.x, top_y, p.y],
            normal: top_normal,
            color: roof_color,
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
            position: [p.x, base_y, p.y],
            normal: bottom_normal,
            color,
        });
    }
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

        let dx = b.x - a.x;
        let dy = b.y - a.y;
        let len = (dx * dx + dy * dy).sqrt();
        let normal = if len > 1e-6 {
            [dy / len, 0.0, -dx / len]
        } else {
            [0.0, 0.0, 1.0]
        };

        let base = vertices.len() as u32;
        vertices.push(Vertex { position: [a.x, base_y, a.y], normal, color });
        vertices.push(Vertex { position: [b.x, base_y, b.y], normal, color });
        vertices.push(Vertex { position: [b.x, top_y, b.y], normal, color });
        vertices.push(Vertex { position: [a.x, top_y, a.y], normal, color });

        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    (vertices, indices)
}

/// Create an L-shaped building footprint (CCW).
pub fn l_shape_footprint(cx: f32, cz: f32, w: f32, d: f32, notch_frac: f32) -> Vec<Vec2> {
    let hw = w / 2.0;
    let hd = d / 2.0;
    let nw = hw * notch_frac;
    let nd = hd * notch_frac;
    vec![
        Vec2::new(cx - hw, cz - hd),
        Vec2::new(cx + hw, cz - hd),
        Vec2::new(cx + hw, cz + hd - nd),
        Vec2::new(cx + hw - nw, cz + hd - nd),
        Vec2::new(cx + hw - nw, cz + hd),
        Vec2::new(cx - hw, cz + hd),
    ]
}

/// Create a simple rectangular footprint (CCW).
pub fn rect_footprint(cx: f32, cz: f32, w: f32, d: f32) -> Vec<Vec2> {
    let hw = w / 2.0;
    let hd = d / 2.0;
    vec![
        Vec2::new(cx - hw, cz - hd),
        Vec2::new(cx + hw, cz - hd),
        Vec2::new(cx + hw, cz + hd),
        Vec2::new(cx - hw, cz + hd),
    ]
}

/// Create a T-shaped building footprint (CCW).
pub fn t_shape_footprint(cx: f32, cz: f32, w: f32, d: f32) -> Vec<Vec2> {
    let hw = w / 2.0;
    let hd = d / 2.0;
    let stem_w = w * 0.35;
    let hsw = stem_w / 2.0;
    let top_d = d * 0.35;
    vec![
        Vec2::new(cx - hsw, cz - hd),
        Vec2::new(cx + hsw, cz - hd),
        Vec2::new(cx + hsw, cz + hd - top_d),
        Vec2::new(cx + hw, cz + hd - top_d),
        Vec2::new(cx + hw, cz + hd),
        Vec2::new(cx - hw, cz + hd),
        Vec2::new(cx - hw, cz + hd - top_d),
        Vec2::new(cx - hsw, cz + hd - top_d),
    ]
}
