use diorama_core::schema::{
    TerrainDef, SlabDef, RoadDef, WaterDef,
    terrain_height_rbf, terrain_color_from_zones, slab_layer_color,
};

use crate::vertex::Vertex;

/// Generate a terrain mesh from a schema definition using Gaussian RBF heights.
pub fn generate_terrain_from_def(def: &TerrainDef) -> (Vec<Vertex>, Vec<u32>) {
    let half_size = def.half_size;
    let resolution = def.resolution;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let step = (half_size * 2.0) / resolution as f32;

    for iz in 0..=resolution {
        for ix in 0..=resolution {
            let x = -half_size + ix as f32 * step;
            let z = -half_size + iz as f32 * step;
            let y = terrain_height_rbf(x, z, &def.control_points);
            let color = terrain_color_from_zones(y, &def.color_zones, def.base_color);

            vertices.push(Vertex {
                position: [x, y, z],
                normal: [0.0, 1.0, 0.0], // placeholder
                color,
            });
        }
    }

    // Compute smooth normals from adjacent vertices
    let w = resolution + 1;
    for iz in 0..=resolution {
        for ix in 0..=resolution {
            let idx = (iz * w + ix) as usize;
            let p = vertices[idx].position;

            let left = if ix > 0 {
                vertices[(iz * w + ix - 1) as usize].position
            } else {
                p
            };
            let right = if ix < resolution {
                vertices[(iz * w + ix + 1) as usize].position
            } else {
                p
            };
            let back = if iz > 0 {
                vertices[((iz - 1) * w + ix) as usize].position
            } else {
                p
            };
            let front = if iz < resolution {
                vertices[((iz + 1) * w + ix) as usize].position
            } else {
                p
            };

            let dx = [right[0] - left[0], right[1] - left[1], right[2] - left[2]];
            let dz = [front[0] - back[0], front[1] - back[1], front[2] - back[2]];

            // Cross product dz x dx gives upward normal
            let nx = dz[1] * dx[2] - dz[2] * dx[1];
            let ny = dz[2] * dx[0] - dz[0] * dx[2];
            let nz = dz[0] * dx[1] - dz[1] * dx[0];
            let len = (nx * nx + ny * ny + nz * nz).sqrt();
            if len > 1e-6 {
                vertices[idx].normal = [nx / len, ny / len, nz / len];
            }
        }
    }

    // Generate triangle indices
    for iz in 0..resolution {
        for ix in 0..resolution {
            let tl = iz * w + ix;
            let tr = tl + 1;
            let bl = (iz + 1) * w + ix;
            let br = bl + 1;

            indices.push(tl);
            indices.push(tr);
            indices.push(bl);

            indices.push(tr);
            indices.push(br);
            indices.push(bl);
        }
    }

    (vertices, indices)
}

/// Generate the thick slab walls and bottom from a schema definition.
pub fn generate_slab_walls_from_def(
    terrain_def: &TerrainDef,
    slab_def: &SlabDef,
) -> (Vec<Vertex>, Vec<u32>) {
    let half_size = terrain_def.half_size;
    let resolution = terrain_def.resolution;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let step = (half_size * 2.0) / resolution as f32;

    // Find the lowest terrain point
    let mut min_y = f32::MAX;
    for iz in 0..=resolution {
        for ix in 0..=resolution {
            let x = -half_size + ix as f32 * step;
            let z = -half_size + iz as f32 * step;
            let y = terrain_height_rbf(x, z, &terrain_def.control_points);
            if y < min_y { min_y = y; }
        }
    }
    let base_y = min_y - slab_def.depth;

    // Bottom color from last layer
    let bottom_color = slab_layer_color(1.0, &slab_def.layers);

    let v_subdivs = 6u32;

    let height_fn = |x: f32, z: f32| terrain_height_rbf(x, z, &terrain_def.control_points);

    // Front edge (z = +half_size, facing +Z)
    for ix in 0..resolution {
        let x0 = -half_size + ix as f32 * step;
        let x1 = x0 + step;
        let z = half_size;
        let y0_top = height_fn(x0, z);
        let y1_top = height_fn(x1, z);
        let normal = [0.0, 0.0, 1.0];

        for iv in 0..v_subdivs {
            let t0 = iv as f32 / v_subdivs as f32;
            let t1 = (iv + 1) as f32 / v_subdivs as f32;
            let y0_a = y0_top + t0 * (base_y - y0_top);
            let y0_b = y0_top + t1 * (base_y - y0_top);
            let y1_a = y1_top + t0 * (base_y - y1_top);
            let y1_b = y1_top + t1 * (base_y - y1_top);
            let color = slab_layer_color((t0 + t1) * 0.5, &slab_def.layers);

            let base = vertices.len() as u32;
            vertices.push(Vertex { position: [x0, y0_a, z], normal, color });
            vertices.push(Vertex { position: [x1, y1_a, z], normal, color });
            vertices.push(Vertex { position: [x1, y1_b, z], normal, color });
            vertices.push(Vertex { position: [x0, y0_b, z], normal, color });

            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 3);
        }
    }

    // Back edge (z = -half_size, facing -Z)
    for ix in 0..resolution {
        let x0 = -half_size + ix as f32 * step;
        let x1 = x0 + step;
        let z = -half_size;
        let y0_top = height_fn(x0, z);
        let y1_top = height_fn(x1, z);
        let normal = [0.0, 0.0, -1.0];

        for iv in 0..v_subdivs {
            let t0 = iv as f32 / v_subdivs as f32;
            let t1 = (iv + 1) as f32 / v_subdivs as f32;
            let y0_a = y0_top + t0 * (base_y - y0_top);
            let y0_b = y0_top + t1 * (base_y - y0_top);
            let y1_a = y1_top + t0 * (base_y - y1_top);
            let y1_b = y1_top + t1 * (base_y - y1_top);
            let color = slab_layer_color((t0 + t1) * 0.5, &slab_def.layers);

            let base = vertices.len() as u32;
            vertices.push(Vertex { position: [x0, y0_a, z], normal, color });
            vertices.push(Vertex { position: [x1, y1_a, z], normal, color });
            vertices.push(Vertex { position: [x1, y1_b, z], normal, color });
            vertices.push(Vertex { position: [x0, y0_b, z], normal, color });

            indices.push(base);
            indices.push(base + 3);
            indices.push(base + 2);
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 1);
        }
    }

    // Right edge (x = +half_size, facing +X)
    for iz in 0..resolution {
        let z0 = -half_size + iz as f32 * step;
        let z1 = z0 + step;
        let x = half_size;
        let y0_top = height_fn(x, z0);
        let y1_top = height_fn(x, z1);
        let normal = [1.0, 0.0, 0.0];

        for iv in 0..v_subdivs {
            let t0 = iv as f32 / v_subdivs as f32;
            let t1 = (iv + 1) as f32 / v_subdivs as f32;
            let y0_a = y0_top + t0 * (base_y - y0_top);
            let y0_b = y0_top + t1 * (base_y - y0_top);
            let y1_a = y1_top + t0 * (base_y - y1_top);
            let y1_b = y1_top + t1 * (base_y - y1_top);
            let color = slab_layer_color((t0 + t1) * 0.5, &slab_def.layers);

            let base = vertices.len() as u32;
            vertices.push(Vertex { position: [x, y0_a, z0], normal, color });
            vertices.push(Vertex { position: [x, y1_a, z1], normal, color });
            vertices.push(Vertex { position: [x, y1_b, z1], normal, color });
            vertices.push(Vertex { position: [x, y0_b, z0], normal, color });

            indices.push(base);
            indices.push(base + 3);
            indices.push(base + 2);
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 1);
        }
    }

    // Left edge (x = -half_size, facing -X)
    for iz in 0..resolution {
        let z0 = -half_size + iz as f32 * step;
        let z1 = z0 + step;
        let x = -half_size;
        let y0_top = height_fn(x, z0);
        let y1_top = height_fn(x, z1);
        let normal = [-1.0, 0.0, 0.0];

        for iv in 0..v_subdivs {
            let t0 = iv as f32 / v_subdivs as f32;
            let t1 = (iv + 1) as f32 / v_subdivs as f32;
            let y0_a = y0_top + t0 * (base_y - y0_top);
            let y0_b = y0_top + t1 * (base_y - y0_top);
            let y1_a = y1_top + t0 * (base_y - y1_top);
            let y1_b = y1_top + t1 * (base_y - y1_top);
            let color = slab_layer_color((t0 + t1) * 0.5, &slab_def.layers);

            let base = vertices.len() as u32;
            vertices.push(Vertex { position: [x, y0_a, z0], normal, color });
            vertices.push(Vertex { position: [x, y1_a, z1], normal, color });
            vertices.push(Vertex { position: [x, y1_b, z1], normal, color });
            vertices.push(Vertex { position: [x, y0_b, z0], normal, color });

            indices.push(base);
            indices.push(base + 1);
            indices.push(base + 2);
            indices.push(base);
            indices.push(base + 2);
            indices.push(base + 3);
        }
    }

    // Bottom face
    let base = vertices.len() as u32;
    vertices.push(Vertex { position: [-half_size, base_y, -half_size], normal: [0.0, -1.0, 0.0], color: bottom_color });
    vertices.push(Vertex { position: [ half_size, base_y, -half_size], normal: [0.0, -1.0, 0.0], color: bottom_color });
    vertices.push(Vertex { position: [ half_size, base_y,  half_size], normal: [0.0, -1.0, 0.0], color: bottom_color });
    vertices.push(Vertex { position: [-half_size, base_y,  half_size], normal: [0.0, -1.0, 0.0], color: bottom_color });
    indices.push(base);
    indices.push(base + 1);
    indices.push(base + 2);
    indices.push(base);
    indices.push(base + 2);
    indices.push(base + 3);

    (vertices, indices)
}

/// Generate flat road strip geometry slightly above terrain.
pub fn generate_road_from_def(
    road_def: &RoadDef,
    terrain_def: &TerrainDef,
) -> (Vec<Vertex>, Vec<u32>) {
    let mut vertices = Vec::new();
    let mut indices = Vec::new();
    let color = [0.42, 0.40, 0.36];

    let points = &road_def.points;
    let width = road_def.width;

    if points.len() < 2 {
        return (vertices, indices);
    }

    for i in 0..points.len() - 1 {
        let ax = points[i][0];
        let az = points[i][1];
        let bx = points[i + 1][0];
        let bz = points[i + 1][1];

        let dx = bx - ax;
        let dz = bz - az;
        let len = (dx * dx + dz * dz).sqrt();
        if len < 1e-6 {
            continue;
        }

        let nx = -dz / len * width * 0.5;
        let nz = dx / len * width * 0.5;

        let road_lift = 0.02;
        let ya = terrain_height_rbf(ax, az, &terrain_def.control_points) + road_lift;
        let yb = terrain_height_rbf(bx, bz, &terrain_def.control_points) + road_lift;

        let base = vertices.len() as u32;
        vertices.push(Vertex { position: [ax + nx, ya, az + nz], normal: [0.0, 1.0, 0.0], color });
        vertices.push(Vertex { position: [ax - nx, ya, az - nz], normal: [0.0, 1.0, 0.0], color });
        vertices.push(Vertex { position: [bx - nx, yb, bz - nz], normal: [0.0, 1.0, 0.0], color });
        vertices.push(Vertex { position: [bx + nx, yb, bz + nz], normal: [0.0, 1.0, 0.0], color });

        indices.push(base);
        indices.push(base + 2);
        indices.push(base + 1);
        indices.push(base);
        indices.push(base + 3);
        indices.push(base + 2);
    }

    (vertices, indices)
}

/// Generate a flat water surface polygon from definition.
pub fn generate_water_from_def(water_def: &WaterDef) -> (Vec<Vertex>, Vec<u32>) {
    let color = [0.25, 0.45, 0.55];
    let segments = 24u32;
    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    let y = -0.15;
    vertices.push(Vertex {
        position: [water_def.cx, y, water_def.cz],
        normal: [0.0, 1.0, 0.0],
        color,
    });

    for i in 0..=segments {
        let angle = (i as f32 / segments as f32) * std::f32::consts::TAU;
        let x = water_def.cx + angle.cos() * water_def.radius_x;
        let z = water_def.cz + angle.sin() * water_def.radius_z;
        vertices.push(Vertex {
            position: [x, y, z],
            normal: [0.0, 1.0, 0.0],
            color,
        });
    }

    for i in 1..=segments {
        indices.push(0);
        indices.push(i + 1);
        indices.push(i);
    }

    (vertices, indices)
}
