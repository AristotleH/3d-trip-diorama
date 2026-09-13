use diorama_core::camera::OrbitCamera;
use diorama_core::schema::DioramaScene;
use wgpu::util::DeviceExt;

use crate::mesh::Mesh;
use crate::pipeline::create_pipeline;
use crate::scene::create_diorama_from_schema;

#[repr(C)]
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable)]
struct Uniforms {
    view_proj: [f32; 16],
    light_dir: [f32; 4], // padded to 16 bytes
}

pub struct RendererState {
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface: wgpu::Surface<'static>,
    surface_config: wgpu::SurfaceConfiguration,
    pipeline: wgpu::RenderPipeline,
    uniform_buffer: wgpu::Buffer,
    bind_group: wgpu::BindGroup,
    depth_texture: wgpu::TextureView,
    color_texture: Option<wgpu::TextureView>,
    sample_count: u32,
    bounds: (f32, f32, f32),
    mesh: Mesh,
    pub camera: OrbitCamera,
}

impl RendererState {
    pub async fn new(canvas: web_sys::HtmlCanvasElement, scene: &DioramaScene) -> Self {
        let (width, height) = (canvas.width().max(1), canvas.height().max(1));
        canvas.set_width(width);
        canvas.set_height(height);

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::BROWSER_WEBGPU,
            ..Default::default()
        });

        let surface_target = wgpu::SurfaceTarget::Canvas(canvas);
        let surface = instance.create_surface(surface_target).unwrap();

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .expect("Failed to find a suitable GPU adapter");

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    memory_hints: Default::default(),
                },
                None,
            )
            .await
            .expect("Failed to create device");

        let surface_caps = surface.get_capabilities(&adapter);
        let format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format,
            width,
            height,
            present_mode: wgpu::PresentMode::Fifo,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        // Uniforms
        let camera = OrbitCamera::new();
        let uniforms = Uniforms {
            view_proj: [0.0; 16],
            light_dir: [0.0; 4],
        };
        let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Uniform Buffer"),
            contents: bytemuck::bytes_of(&uniforms),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Bind Group Layout"),
            entries: &[wgpu::BindGroupLayoutEntry {
                binding: 0,
                visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                ty: wgpu::BindingType::Buffer {
                    ty: wgpu::BufferBindingType::Uniform,
                    has_dynamic_offset: false,
                    min_binding_size: None,
                },
                count: None,
            }],
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Bind Group"),
            layout: &bind_group_layout,
            entries: &[wgpu::BindGroupEntry {
                binding: 0,
                resource: uniform_buffer.as_entire_binding(),
            }],
        });

        let sample_count = if adapter.get_texture_format_features(format).flags.sample_count_supported(4)
            && adapter.get_texture_format_features(wgpu::TextureFormat::Depth32Float).flags.sample_count_supported(4) { 4 } else { 1 };
        let pipeline = create_pipeline(&device, format, &bind_group_layout, sample_count);
        let depth_texture = Self::create_depth_texture(&device, width, height, sample_count);
        let color_texture = Self::create_color_texture(&device, width, height, format, sample_count);
        let mesh = create_diorama_from_schema(&device, scene);

        let mut state = Self {
            device,
            queue,
            surface,
            surface_config,
            pipeline,
            uniform_buffer,
            bind_group,
            depth_texture,
            color_texture,
            sample_count,
            bounds: (scene.terrain.half_size, -scene.slab.depth, 0.0),
            mesh,
            camera,
        };
        state.frame_scene(scene);
        state
    }

    pub fn set_scene(&mut self, scene: &DioramaScene) {
        self.mesh = create_diorama_from_schema(&self.device, scene);
        self.frame_scene(scene);
    }

    fn frame_scene(&mut self, scene: &DioramaScene) {
        self.camera = OrbitCamera::new();
        let height = scene.buildings.iter().map(|b| b.height).fold(0.0_f32, f32::max);
        let low = scene.terrain.control_points.iter().map(|p| p.amplitude.min(0.0)).sum::<f32>() - scene.slab.depth;
        let high = scene.terrain.control_points.iter().map(|p| p.amplitude.max(0.0)).sum::<f32>() + height + 0.18;
        self.bounds = (scene.terrain.half_size, low, high);
        let aspect = self.surface_config.width as f32 / self.surface_config.height as f32;
        self.camera.fit_bounds(self.bounds.0, low, high, aspect);
    }

    pub fn mesh_index_count(&self) -> u32 {
        self.mesh.index_count
    }

    pub fn surface_size(&self) -> (u32, u32) {
        (self.surface_config.width, self.surface_config.height)
    }

    fn create_depth_texture(device: &wgpu::Device, width: u32, height: u32, sample_count: u32) -> wgpu::TextureView {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Depth Texture"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1,
            sample_count,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::Depth32Float,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            view_formats: &[],
        });
        texture.create_view(&wgpu::TextureViewDescriptor::default())
    }

    fn create_color_texture(device: &wgpu::Device, width: u32, height: u32, format: wgpu::TextureFormat, sample_count: u32) -> Option<wgpu::TextureView> {
        if sample_count == 1 { return None; }
        Some(device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Antialiasing Color"),
            size: wgpu::Extent3d { width, height, depth_or_array_layers: 1 },
            mip_level_count: 1, sample_count,
            dimension: wgpu::TextureDimension::D2, format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT, view_formats: &[],
        }).create_view(&Default::default()))
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 || (width, height) == self.surface_size() {
            return;
        }
        let old_distance = self.camera.distance;
        let (half, low, high) = self.bounds;
        self.camera.fit_bounds(half, low, high, self.surface_config.width as f32 / self.surface_config.height as f32);
        let zoom = old_distance / self.camera.distance;
        self.camera.fit_bounds(half, low, high, width as f32 / height as f32);
        self.camera.distance = (self.camera.distance * zoom).clamp(5.0, self.camera.max_distance);
        self.surface_config.width = width;
        self.surface_config.height = height;
        self.surface.configure(&self.device, &self.surface_config);
        self.depth_texture = Self::create_depth_texture(&self.device, width, height, self.sample_count);
        self.color_texture = Self::create_color_texture(&self.device, width, height, self.surface_config.format, self.sample_count);
    }

    pub fn render(&self) {
        let aspect = self.surface_config.width as f32 / self.surface_config.height as f32;
        let view = self.camera.view_matrix();
        let proj = self.camera.projection_matrix(aspect);
        let view_proj = proj * view;

        let light_dir = diorama_core::math::Vec3::new(0.4, -0.8, -0.3).normalize();
        let uniforms = Uniforms {
            view_proj: view_proj.to_cols_array(),
            light_dir: [light_dir.x, light_dir.y, light_dir.z, 0.0],
        };
        self.queue.write_buffer(&self.uniform_buffer, 0, bytemuck::bytes_of(&uniforms));

        let output = match self.surface.get_current_texture() {
            Ok(t) => t,
            Err(_) => return,
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });

        {
            let mut pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: self.color_texture.as_ref().unwrap_or(&view),
                    resolve_target: self.color_texture.as_ref().map(|_| &view),
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.53,
                            g: 0.72,
                            b: 0.85,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: wgpu::StoreOp::Store,
                    }),
                    stencil_ops: None,
                }),
                timestamp_writes: None,
                occlusion_query_set: None,
            });
            pass.set_pipeline(&self.pipeline);
            pass.set_bind_group(0, &self.bind_group, &[]);
            self.mesh.draw(&mut pass);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    }
}
