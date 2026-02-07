use diorama_core::schema::{
    DioramaScene, BuildingShape, TreeDef, terrain_height_rbf,
};
use crate::building::{extrude_building, l_shape_footprint, rect_footprint, t_shape_footprint};
use crate::mesh::Mesh;
use crate::terrain::{
    generate_terrain_from_def, generate_slab_walls_from_def,
    generate_road_from_def, generate_water_from_def,
};
use crate::vertex::Vertex;

fn add_geometry(
    all_verts: &mut Vec<Vertex>,
    all_idxs: &mut Vec<u32>,
    verts: &[Vertex],
    idxs: &[u32],
) {
    let base = all_verts.len() as u32;
    all_verts.extend_from_slice(verts);
    for i in idxs {
        all_idxs.push(base + i);
    }
}

pub fn create_diorama_from_schema(device: &wgpu::Device, scene: &DioramaScene) -> Mesh {
    let mut all_vertices: Vec<Vertex> = Vec::new();
    let mut all_indices: Vec<u32> = Vec::new();

    // --- Terrain ---
    let (tv, ti) = generate_terrain_from_def(&scene.terrain);
    add_geometry(&mut all_vertices, &mut all_indices, &tv, &ti);

    // --- Slab walls ---
    let (sv, si) = generate_slab_walls_from_def(&scene.terrain, &scene.slab);
    add_geometry(&mut all_vertices, &mut all_indices, &sv, &si);

    // --- Water features ---
    for water_def in &scene.water {
        let (wv, wi) = generate_water_from_def(water_def);
        add_geometry(&mut all_vertices, &mut all_indices, &wv, &wi);
    }

    // --- Roads ---
    for road_def in &scene.roads {
        let (rv, ri) = generate_road_from_def(road_def, &scene.terrain);
        add_geometry(&mut all_vertices, &mut all_indices, &rv, &ri);
    }

    // --- Buildings ---
    for b in &scene.buildings {
        let footprint = match b.shape {
            BuildingShape::LShape => l_shape_footprint(b.cx, b.cz, b.width, b.depth, 0.45),
            BuildingShape::TShape => t_shape_footprint(b.cx, b.cz, b.width, b.depth),
            BuildingShape::Rect => rect_footprint(b.cx, b.cz, b.width, b.depth),
        };
        let base_y = terrain_height_rbf(b.cx, b.cz, &scene.terrain.control_points);
        let (verts, idxs) = extrude_building(&footprint, b.height, base_y, b.color);
        add_geometry(&mut all_vertices, &mut all_indices, &verts, &idxs);
    }

    // --- Trees ---
    for tree_def in &scene.trees {
        let base_y = terrain_height_rbf(tree_def.x, tree_def.z, &scene.terrain.control_points);
        let (tv, ti) = make_tree(tree_def, base_y);
        add_geometry(&mut all_vertices, &mut all_indices, &tv, &ti);
    }

    Mesh::new(device, &all_vertices, &all_indices)
}

/// Simple tree: brown cylinder trunk + green cone canopy.
fn make_tree(def: &TreeDef, base_y: f32) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let x = def.x;
    let z = def.z;
    let trunk_color = def.trunk_color;
    let leaf_color = def.leaf_color;
    let trunk_r = def.trunk_radius;
    let trunk_h = def.trunk_height;
    let canopy_r = def.canopy_radius;
    let canopy_h = def.canopy_height;
    let segments = 8u32;

    // Trunk (octagonal prism)
    for i in 0..segments {
        let a0 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let (c0, s0) = (a0.cos(), a0.sin());
        let (c1, s1) = (a1.cos(), a1.sin());

        let base = vertices.len() as u32;
        let nx = (c0 + c1) * 0.5;
        let nz = (s0 + s1) * 0.5;
        let nl = (nx * nx + nz * nz).sqrt();
        let normal = [nx / nl, 0.0, nz / nl];

        vertices.push(Vertex { position: [x + c0 * trunk_r, base_y, z + s0 * trunk_r], normal, color: trunk_color });
        vertices.push(Vertex { position: [x + c1 * trunk_r, base_y, z + s1 * trunk_r], normal, color: trunk_color });
        vertices.push(Vertex { position: [x + c1 * trunk_r, base_y + trunk_h, z + s1 * trunk_r], normal, color: trunk_color });
        vertices.push(Vertex { position: [x + c0 * trunk_r, base_y + trunk_h, z + s0 * trunk_r], normal, color: trunk_color });

        indices.push(base);
        indices.push(base + 1);
        indices.push(base + 2);
        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 3);
    }

    // Canopy (cone)
    let cone_base_y = base_y + trunk_h * 0.5;
    let cone_tip_y = cone_base_y + canopy_h;
    let tip_idx = vertices.len() as u32;
    vertices.push(Vertex {
        position: [x, cone_tip_y, z],
        normal: [0.0, 1.0, 0.0],
        color: leaf_color,
    });

    for i in 0..segments {
        let a0 = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let a1 = ((i + 1) as f32 / segments as f32) * std::f32::consts::TAU;
        let (c0, s0) = (a0.cos(), a0.sin());
        let (c1, s1) = (a1.cos(), a1.sin());

        let mx = (c0 + c1) * 0.5;
        let mz = (s0 + s1) * 0.5;
        let slope = canopy_r / canopy_h;
        let ny = slope;
        let nl = (mx * mx + mz * mz + ny * ny).sqrt();
        let normal = [mx / nl, ny / nl, mz / nl];

        let base = vertices.len() as u32;
        vertices.push(Vertex { position: [x + c0 * canopy_r, cone_base_y, z + s0 * canopy_r], normal, color: leaf_color });
        vertices.push(Vertex { position: [x + c1 * canopy_r, cone_base_y, z + s1 * canopy_r], normal, color: leaf_color });

        indices.push(tip_idx);
        indices.push(base);
        indices.push(base + 1);
    }

    (vertices, indices)
}
