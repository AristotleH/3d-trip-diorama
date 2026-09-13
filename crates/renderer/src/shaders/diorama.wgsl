struct Uniforms {
    view_proj: mat4x4<f32>,
    light_dir: vec4<f32>,
}

@group(0) @binding(0)
var<uniform> uniforms: Uniforms;

struct VertexInput {
    @location(0) position: vec3<f32>,
    @location(1) normal: vec3<f32>,
    @location(2) color: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) material: u32,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec3<f32>,
    @location(2) world_position: vec3<f32>,
    @location(3) uv: vec2<f32>,
    @location(4) @interpolate(flat) material: u32,
}

@vertex
fn vs_main(vin: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(vin.position, 1.0);
    out.world_normal = vin.normal;
    out.color = vin.color;
    out.world_position = vin.position;
    out.uv = vin.uv;
    out.material = vin.material;
    return out;
}

fn hash21(p: vec2<f32>) -> f32 {
    let q = fract(p * vec2<f32>(123.34, 456.21));
    return fract((q.x + q.y) * (q.x + q.y + 45.32));
}

fn value_noise(p: vec2<f32>) -> f32 {
    let i = floor(p);
    let f = fract(p);
    let u = f * f * (3.0 - 2.0 * f);
    return mix(mix(hash21(i), hash21(i + vec2<f32>(1.0, 0.0)), u.x),
               mix(hash21(i + vec2<f32>(0.0, 1.0)), hash21(i + vec2<f32>(1.0, 1.0)), u.x), u.y);
}

fn terrain_material(base: vec3<f32>, p: vec3<f32>) -> vec3<f32> {
    let broad = value_noise(p.xz * 0.22);
    let medium = value_noise(p.xz * 0.85 + 17.0);
    let fine = value_noise(p.xz * 4.5 + 39.0);
    let variation = (broad - 0.5) * 0.18 + (medium - 0.5) * 0.10 + (fine - 0.5) * 0.035;
    let warm_soil = vec3<f32>(0.34, 0.25, 0.14);
    let sparse = smoothstep(0.69, 0.88, broad + medium * 0.22);
    var color = base * (1.0 + variation);
    color = mix(color, warm_soil, sparse * 0.18);
    // Tiny deterministic flecks read as grass/aggregate without texture assets.
    let fleck = step(0.965, hash21(floor(p.xz * 7.0)));
    return mix(color, color * vec3<f32>(0.68, 0.82, 0.58), fleck * 0.28);
}

const FACADE_BAY: vec2<f32> = vec2<f32>(0.92, 1.35);

fn facade_material(base: vec3<f32>, uv: vec2<f32>, p: vec3<f32>, edge: vec2<f32>) -> vec3<f32> {
    // Metre-based bays give every footprint aligned floors and windows.
    let bay = FACADE_BAY;
    let cell = fract(uv / bay);
    let wx = smoothstep(0.15 - edge.x, 0.15 + edge.x, cell.x) *
             (1.0 - smoothstep(0.85 - edge.x, 0.85 + edge.x, cell.x));
    let wy = smoothstep(0.22 - edge.y, 0.22 + edge.y, cell.y) *
             (1.0 - smoothstep(0.78 - edge.y, 0.78 + edge.y, cell.y));
    let window = wx * wy * step(0.65, uv.y);
    let cell_id = floor(uv / bay);
    let room = hash21(cell_id + floor(p.xz * 0.1));
    let glass = mix(vec3<f32>(0.12, 0.19, 0.23), vec3<f32>(0.48, 0.58, 0.55), room * 0.42);
    let masonry = base * (0.94 + 0.08 * value_noise(p.xz * 1.7 + uv.yy));
    let floor_band = 1.0 - smoothstep(0.04, 0.09, min(cell.y, 1.0 - cell.y));
    let framed = mix(masonry, masonry * 0.72, floor_band * 0.16);
    return mix(framed, glass, window * 0.88);
}

fn roof_material(base: vec3<f32>, uv: vec2<f32>) -> vec3<f32> {
    let membrane = value_noise(uv * 1.7) * 0.10 + value_noise(uv * 6.0) * 0.035;
    let seams_x = 1.0 - smoothstep(0.015, 0.045, abs(fract(uv.x * 0.55) - 0.5));
    let seams_z = 1.0 - smoothstep(0.015, 0.045, abs(fract(uv.y * 0.55) - 0.5));
    return base * (0.91 + membrane) * (1.0 - max(seams_x, seams_z) * 0.045);
}

@fragment
fn fs_main(fin: VertexOutput) -> @location(0) vec4<f32> {
    // Derivatives must execute uniformly, before material-dependent branches.
    // A flat material ID is not guaranteed uniform across a fragment quad.
    let facade_edge = fwidth(fin.uv / FACADE_BAY);
    let normal = normalize(fin.world_normal);
    let light_dir = normalize(-uniforms.light_dir.xyz);
    var surface = fin.color;
    var ambient = 0.34;
    var diffuse_strength = 0.66;

    if fin.material == 0u { surface = terrain_material(surface, fin.world_position); }
    if fin.material == 1u { surface = facade_material(surface, fin.uv, fin.world_position, facade_edge); }
    if fin.material == 2u { surface = roof_material(surface, fin.uv); }
    if fin.material == 3u {
        let strata = 0.94 + 0.07 * value_noise(vec2<f32>(fin.world_position.x * 0.8, fin.world_position.y * 3.0));
        surface *= strata;
    }
    if fin.material == 4u {
        let aggregate = value_noise(fin.world_position.xz * 7.0);
        surface *= 0.91 + aggregate * 0.13;
    }
    if fin.material == 5u {
        let ripple = sin(fin.world_position.x * 2.1 + value_noise(fin.world_position.xz) * 2.0) * 0.035;
        surface += vec3<f32>(ripple, ripple, ripple * 1.5);
        ambient = 0.48;
        diffuse_strength = 0.52;
    }
    if fin.material == 6u { surface *= 0.86 + value_noise(fin.world_position.xy * 7.0) * 0.18; }
    if fin.material == 7u { surface *= 0.84 + value_noise(fin.world_position.xz * 4.0) * 0.25; }

    let ndotl = max(dot(normal, light_dir), 0.0);
    let sky = max(normal.y, 0.0) * 0.08;
    let brightness = ambient + ndotl * diffuse_strength + sky;
    return vec4<f32>(clamp(surface * brightness, vec3<f32>(0.0), vec3<f32>(1.0)), 1.0);
}
