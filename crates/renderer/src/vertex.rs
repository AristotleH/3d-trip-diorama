use bytemuck::{Pod, Zeroable};

pub const MATERIAL_TERRAIN: u32 = 0;
pub const MATERIAL_BUILDING_WALL: u32 = 1;
pub const MATERIAL_BUILDING_ROOF: u32 = 2;
pub const MATERIAL_EARTH: u32 = 3;
pub const MATERIAL_ROAD: u32 = 4;
pub const MATERIAL_WATER: u32 = 5;
pub const MATERIAL_TRUNK: u32 = 6;
pub const MATERIAL_FOLIAGE: u32 = 7;

#[repr(C)]
#[derive(Clone, Copy, Debug, Pod, Zeroable)]
pub struct Vertex {
    pub position: [f32; 3],
    pub normal: [f32; 3],
    pub color: [f32; 3],
    /// Material-space coordinates. Buildings use metres along/across a surface;
    /// landscape materials use world XZ so details remain stable across meshes.
    pub uv: [f32; 2],
    pub material: u32,
}

impl Vertex {
    pub fn new(
        position: [f32; 3],
        normal: [f32; 3],
        color: [f32; 3],
        uv: [f32; 2],
        material: u32,
    ) -> Self {
        Self { position, normal, color, uv, material }
    }

    pub fn layout() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttribute { offset: 0, shader_location: 0, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 12, shader_location: 1, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 24, shader_location: 2, format: wgpu::VertexFormat::Float32x3 },
                wgpu::VertexAttribute { offset: 36, shader_location: 3, format: wgpu::VertexFormat::Float32x2 },
                wgpu::VertexAttribute { offset: 44, shader_location: 4, format: wgpu::VertexFormat::Uint32 },
            ],
        }
    }
}
