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
}

struct VertexOutput {
    @builtin(position) clip_position: vec4<f32>,
    @location(0) world_normal: vec3<f32>,
    @location(1) color: vec3<f32>,
}

@vertex
fn vs_main(vin: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_position = uniforms.view_proj * vec4<f32>(vin.position, 1.0);
    out.world_normal = vin.normal;
    out.color = vin.color;
    return out;
}

@fragment
fn fs_main(fin: VertexOutput) -> @location(0) vec4<f32> {
    let normal = normalize(fin.world_normal);
    let light_dir = normalize(-uniforms.light_dir.xyz);

    let ndotl = max(dot(normal, light_dir), 0.0);
    let ambient = 0.3;
    let diffuse = ndotl * 0.7;
    let brightness = ambient + diffuse;

    let lit_color = fin.color * brightness;
    return vec4<f32>(lit_color, 1.0);
}
