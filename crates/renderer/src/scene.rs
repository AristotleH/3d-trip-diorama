use diorama_core::math::Vec2;

use crate::building::extrude_building;
use crate::mesh::Mesh;
use crate::vertex::Vertex;

/// Warm color palette for buildings.
const BUILDING_COLORS: [[f32; 3]; 8] = [
    [0.93, 0.78, 0.62], // warm sand
    [0.87, 0.63, 0.47], // terra cotta
    [0.95, 0.87, 0.73], // cream
    [0.82, 0.71, 0.55], // khaki
    [0.90, 0.70, 0.50], // peach
    [0.85, 0.80, 0.70], // beige
    [0.78, 0.60, 0.45], // sienna
    [0.92, 0.85, 0.78], // linen
];

struct BuildingSpec {
    cx: f32,
    cz: f32,
    w: f32,
    d: f32,
    h: f32,
    color_idx: usize,
}

pub fn create_diorama(device: &wgpu::Device) -> Mesh {
    let mut all_vertices: Vec<Vertex> = Vec::new();
    let mut all_indices: Vec<u32> = Vec::new();

    // Ground plane (20x20, centered at origin)
    let ground_color = [0.55, 0.65, 0.45]; // muted green
    let gs = 15.0_f32;
    let gv = vec![
        Vertex { position: [-gs, 0.0, -gs], normal: [0.0, 1.0, 0.0], color: ground_color },
        Vertex { position: [ gs, 0.0, -gs], normal: [0.0, 1.0, 0.0], color: ground_color },
        Vertex { position: [ gs, 0.0,  gs], normal: [0.0, 1.0, 0.0], color: ground_color },
        Vertex { position: [-gs, 0.0,  gs], normal: [0.0, 1.0, 0.0], color: ground_color },
    ];
    let gi: Vec<u32> = vec![0, 1, 2, 0, 2, 3];
    let base = all_vertices.len() as u32;
    all_vertices.extend_from_slice(&gv);
    for i in &gi {
        all_indices.push(base + i);
    }

    // Procedural buildings in a grid-like layout with variation
    let buildings = vec![
        // Row 1 (back)
        BuildingSpec { cx: -10.0, cz: -10.0, w: 2.5, d: 2.5, h: 6.0, color_idx: 0 },
        BuildingSpec { cx:  -6.0, cz:  -9.0, w: 3.0, d: 2.0, h: 4.5, color_idx: 1 },
        BuildingSpec { cx:  -2.0, cz: -10.5, w: 2.0, d: 3.0, h: 8.0, color_idx: 2 },
        BuildingSpec { cx:   2.5, cz:  -9.5, w: 2.5, d: 2.5, h: 5.0, color_idx: 3 },
        BuildingSpec { cx:   7.0, cz: -10.0, w: 3.0, d: 2.0, h: 7.0, color_idx: 4 },
        BuildingSpec { cx:  11.0, cz:  -9.0, w: 2.0, d: 2.5, h: 3.5, color_idx: 5 },
        // Row 2
        BuildingSpec { cx: -11.0, cz: -5.0, w: 2.0, d: 2.0, h: 5.5, color_idx: 6 },
        BuildingSpec { cx:  -7.0, cz: -4.5, w: 2.5, d: 3.0, h: 3.0, color_idx: 7 },
        BuildingSpec { cx:  -1.5, cz: -5.5, w: 3.5, d: 2.0, h: 9.0, color_idx: 0 },
        BuildingSpec { cx:   3.0, cz: -4.0, w: 2.0, d: 2.0, h: 4.0, color_idx: 2 },
        BuildingSpec { cx:   8.0, cz: -5.0, w: 2.5, d: 2.5, h: 6.5, color_idx: 1 },
        // Row 3
        BuildingSpec { cx:  -9.0, cz:  0.5, w: 2.0, d: 3.0, h: 4.0, color_idx: 3 },
        BuildingSpec { cx:  -4.0, cz:  0.0, w: 3.0, d: 2.5, h: 7.5, color_idx: 5 },
        BuildingSpec { cx:   1.0, cz:  1.0, w: 2.5, d: 2.5, h: 5.0, color_idx: 4 },
        BuildingSpec { cx:   6.0, cz:  0.5, w: 2.0, d: 2.0, h: 3.5, color_idx: 6 },
        BuildingSpec { cx:  10.0, cz: -0.5, w: 2.5, d: 3.0, h: 6.0, color_idx: 7 },
        // Row 4 (front)
        BuildingSpec { cx: -10.0, cz:  5.5, w: 2.5, d: 2.0, h: 4.5, color_idx: 1 },
        BuildingSpec { cx:  -5.5, cz:  6.0, w: 3.0, d: 2.5, h: 6.0, color_idx: 0 },
        BuildingSpec { cx:  -0.5, cz:  5.0, w: 2.0, d: 2.0, h: 8.5, color_idx: 3 },
        BuildingSpec { cx:   4.0, cz:  6.5, w: 2.5, d: 3.0, h: 3.0, color_idx: 2 },
        BuildingSpec { cx:   8.5, cz:  5.5, w: 2.0, d: 2.5, h: 5.5, color_idx: 5 },
        // Row 5
        BuildingSpec { cx:  -8.0, cz: 10.0, w: 2.5, d: 2.0, h: 4.0, color_idx: 7 },
        BuildingSpec { cx:  -3.0, cz: 10.5, w: 2.0, d: 2.5, h: 7.0, color_idx: 4 },
        BuildingSpec { cx:   2.0, cz: 11.0, w: 3.0, d: 2.0, h: 5.0, color_idx: 6 },
        BuildingSpec { cx:   7.5, cz: 10.0, w: 2.0, d: 2.0, h: 6.5, color_idx: 0 },
    ];

    for b in &buildings {
        let hw = b.w / 2.0;
        let hd = b.d / 2.0;
        let footprint = vec![
            Vec2::new(b.cx - hw, b.cz - hd),
            Vec2::new(b.cx + hw, b.cz - hd),
            Vec2::new(b.cx + hw, b.cz + hd),
            Vec2::new(b.cx - hw, b.cz + hd),
        ];
        let color = BUILDING_COLORS[b.color_idx % BUILDING_COLORS.len()];
        let (verts, idxs) = extrude_building(&footprint, b.h, color);
        let base = all_vertices.len() as u32;
        all_vertices.extend_from_slice(&verts);
        for i in &idxs {
            all_indices.push(base + i);
        }
    }

    Mesh::new(device, &all_vertices, &all_indices)
}
