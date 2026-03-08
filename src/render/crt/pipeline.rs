use crate::config::Config;
use bytemuck::{Pod, Zeroable};

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct CrtUniforms {
    resolution: [f32; 2],
    time: f32,
    curvature: f32,
    scanline_intensity: f32,
    scanline_count: f32,
    bloom_intensity: f32,
    bloom_radius: f32,
    chromatic_aberration: f32,
    vignette_intensity: f32,
    flicker_intensity: f32,
    noise: f32,
    phosphor_persistence: f32,
    bezel_size: f32,
    screen_brightness: f32,
    tab_bar_offset: f32,
    bezel_color: [f32; 3],
    opacity: f32,
    bezel_corner_radius: f32,
    _pad: [f32; 3], // Pad to 16-byte alignment (96 bytes total = 24 floats)
}

pub struct CrtPipeline {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    input_texture: wgpu::Texture,
    input_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    width: u32,
    height: u32,
    format: wgpu::TextureFormat,
}

impl CrtPipeline {
    pub fn new(
        device: &wgpu::Device,
        format: wgpu::TextureFormat,
        width: u32,
        height: u32,
    ) -> Self {
        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("CRT Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../../shaders/crt.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("CRT Bind Group Layout"),
            entries: &[
                // Uniforms
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
                // Input texture
                wgpu::BindGroupLayoutEntry {
                    binding: 1,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Texture {
                        sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        view_dimension: wgpu::TextureViewDimension::D2,
                        multisampled: false,
                    },
                    count: None,
                },
                // Sampler
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("CRT Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("CRT Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: None,
                polygon_mode: wgpu::PolygonMode::Fill,
                unclipped_depth: false,
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("CRT Uniform Buffer"),
            size: std::mem::size_of::<CrtUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let (input_texture, input_view) = Self::create_input_texture(device, width, height, format);

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("CRT Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            address_mode_w: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            mipmap_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let bind_group = Self::create_bind_group(
            device,
            &bind_group_layout,
            &uniform_buffer,
            &input_view,
            &sampler,
        );

        Self {
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            input_texture,
            input_view,
            sampler,
            width,
            height,
            format,
        }
    }

    fn create_input_texture(
        device: &wgpu::Device,
        width: u32,
        height: u32,
        format: wgpu::TextureFormat,
    ) -> (wgpu::Texture, wgpu::TextureView) {
        let texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("CRT Input Texture"),
            size: wgpu::Extent3d {
                width: width.max(1),
                height: height.max(1),
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format,
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT | wgpu::TextureUsages::TEXTURE_BINDING,
            view_formats: &[],
        });

        let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        (texture, view)
    }

    fn create_bind_group(
        device: &wgpu::Device,
        layout: &wgpu::BindGroupLayout,
        uniform_buffer: &wgpu::Buffer,
        input_view: &wgpu::TextureView,
        sampler: &wgpu::Sampler,
    ) -> wgpu::BindGroup {
        device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("CRT Bind Group"),
            layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(input_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(sampler),
                },
            ],
        })
    }

    pub fn resize(&mut self, device: &wgpu::Device, _format: wgpu::TextureFormat, width: u32, height: u32) {
        if width == self.width && height == self.height {
            return;
        }

        self.width = width;
        self.height = height;

        let (input_texture, input_view) = Self::create_input_texture(device, width, height, self.format);
        self.input_texture = input_texture;
        self.input_view = input_view;

        self.bind_group = Self::create_bind_group(
            device,
            &self.bind_group_layout,
            &self.uniform_buffer,
            &self.input_view,
            &self.sampler,
        );
    }

    pub fn input_view(&self) -> &wgpu::TextureView {
        &self.input_view
    }

    pub fn render(
        &self,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        config: &Config,
        frame_count: u64,
        tab_bar_height: f32,
    ) {
        let time = (frame_count as f32) / 60.0;
        let enabled = config.crt.enabled;

        let uniforms = CrtUniforms {
            resolution: [self.width as f32, self.height as f32],
            time,
            curvature: if enabled { config.crt.curvature } else { 0.0 },
            scanline_intensity: if enabled { config.crt.scanline_intensity } else { 0.0 },
            scanline_count: config.crt.scanline_count,
            bloom_intensity: if enabled { config.crt.bloom_intensity } else { 0.0 },
            bloom_radius: config.crt.bloom_radius,
            chromatic_aberration: if enabled { config.crt.chromatic_aberration } else { 0.0 },
            vignette_intensity: if enabled { config.crt.vignette_intensity } else { 0.0 },
            flicker_intensity: if enabled && config.crt.flicker { config.crt.flicker_intensity } else { 0.0 },
            noise: if enabled { config.crt.noise } else { 0.0 },
            phosphor_persistence: config.crt.phosphor_persistence,
            bezel_size: if enabled { config.crt.bezel_size } else { 0.0 },
            screen_brightness: config.crt.screen_brightness,
            tab_bar_offset: tab_bar_height / self.height as f32,
            bezel_color: config.crt.bezel_color,
            opacity: config.appearance.opacity,
            bezel_corner_radius: config.crt.bezel_corner_radius,
            _pad: [0.0; 3],
        };

        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("CRT Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color::TRANSPARENT),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.draw(0..6, 0..1);
    }
}
