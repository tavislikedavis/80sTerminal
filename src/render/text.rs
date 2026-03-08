use crate::config::{Colors, Config};
use crate::terminal::{CellColor, Grid, Selection};
use bytemuck::{Pod, Zeroable};
use fontdue::{Font, FontSettings};
use std::collections::HashMap;

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct TextUniforms {
    resolution: [f32; 2],
    cell_size: [f32; 2],
    atlas_size: [f32; 2],
    _padding: [f32; 2],
}

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
struct GlyphVertex {
    position: [f32; 2],
    uv: [f32; 2],
    color: [f32; 4],
    bg_color: [f32; 4],
}

struct GlyphInfo {
    uv_min: [f32; 2],
    uv_max: [f32; 2],
    offset: [f32; 2],
    advance: f32,
}

pub struct TextRenderer {
    pipeline: wgpu::RenderPipeline,
    bind_group_layout: wgpu::BindGroupLayout,
    bind_group: wgpu::BindGroup,
    uniform_buffer: wgpu::Buffer,
    vertex_buffer: wgpu::Buffer,
    atlas_texture: wgpu::Texture,
    atlas_view: wgpu::TextureView,
    sampler: wgpu::Sampler,
    font: Font,
    font_size: f32,
    ascent: f32,
    cell_width: f32,
    cell_height: f32,
    glyph_cache: HashMap<char, GlyphInfo>,
    atlas_width: u32,
    atlas_height: u32,
    atlas_cursor_x: u32,
    atlas_cursor_y: u32,
    atlas_row_height: u32,
    vertex_data: Vec<GlyphVertex>,
    max_vertices: usize,
    width: u32,
    height: u32,
    opacity: f32,
}

impl TextRenderer {
    pub fn new(
        device: &wgpu::Device,
        queue: &wgpu::Queue,
        format: wgpu::TextureFormat,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let font_size = config.appearance.font_size;

        // Load a built-in monospace font or system font
        let font_data = include_bytes!("../assets/JetBrainsMono-Regular.ttf");
        let font = Font::from_bytes(font_data as &[u8], FontSettings::default())
            .map_err(|e| anyhow::anyhow!("Failed to load font: {}", e))?;

        // Calculate cell dimensions and ascent
        let metrics = font.metrics('M', font_size);
        let cell_width = metrics.advance_width.ceil();
        let cell_height = (font_size * config.appearance.line_height).ceil();

        // Get line metrics for proper baseline positioning
        let line_metrics = font.horizontal_line_metrics(font_size).unwrap_or(
            fontdue::LineMetrics {
                ascent: font_size * 0.8,
                descent: font_size * -0.2,
                line_gap: 0.0,
                new_line_size: font_size,
            }
        );
        let ascent = line_metrics.ascent;

        // Create glyph atlas texture
        let atlas_width = 1024u32;
        let atlas_height = 1024u32;
        let atlas_texture = device.create_texture(&wgpu::TextureDescriptor {
            label: Some("Glyph Atlas"),
            size: wgpu::Extent3d {
                width: atlas_width,
                height: atlas_height,
                depth_or_array_layers: 1,
            },
            mip_level_count: 1,
            sample_count: 1,
            dimension: wgpu::TextureDimension::D2,
            format: wgpu::TextureFormat::R8Unorm,
            usage: wgpu::TextureUsages::TEXTURE_BINDING | wgpu::TextureUsages::COPY_DST,
            view_formats: &[],
        });
        let atlas_view = atlas_texture.create_view(&wgpu::TextureViewDescriptor::default());

        let sampler = device.create_sampler(&wgpu::SamplerDescriptor {
            label: Some("Text Sampler"),
            address_mode_u: wgpu::AddressMode::ClampToEdge,
            address_mode_v: wgpu::AddressMode::ClampToEdge,
            mag_filter: wgpu::FilterMode::Linear,
            min_filter: wgpu::FilterMode::Linear,
            ..Default::default()
        });

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Text Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/text.wgsl").into()),
        });

        let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            label: Some("Text Bind Group Layout"),
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                },
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
                wgpu::BindGroupLayoutEntry {
                    binding: 2,
                    visibility: wgpu::ShaderStages::FRAGMENT,
                    ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                    count: None,
                },
            ],
        });

        let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Text Pipeline Layout"),
            bind_group_layouts: &[&bind_group_layout],
            push_constant_ranges: &[],
        });

        let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Text Pipeline"),
            layout: Some(&pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main",
                buffers: &[wgpu::VertexBufferLayout {
                    array_stride: std::mem::size_of::<GlyphVertex>() as u64,
                    step_mode: wgpu::VertexStepMode::Vertex,
                    attributes: &[
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 0,
                            shader_location: 0,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x2,
                            offset: 8,
                            shader_location: 1,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 16,
                            shader_location: 2,
                        },
                        wgpu::VertexAttribute {
                            format: wgpu::VertexFormat::Float32x4,
                            offset: 32,
                            shader_location: 3,
                        },
                    ],
                }],
                compilation_options: Default::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: "fs_main",
                targets: &[Some(wgpu::ColorTargetState {
                    format,
                    blend: Some(wgpu::BlendState::ALPHA_BLENDING),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: Default::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                ..Default::default()
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState::default(),
            multiview: None,
        });

        let uniform_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Uniform Buffer"),
            size: std::mem::size_of::<TextUniforms>() as u64,
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        // Pre-allocate vertex buffer for terminal grid (support up to 4K resolution)
        let max_vertices = 300 * 300 * 6; // rows * cols * vertices per quad
        let vertex_buffer = device.create_buffer(&wgpu::BufferDescriptor {
            label: Some("Text Vertex Buffer"),
            size: (max_vertices * std::mem::size_of::<GlyphVertex>()) as u64,
            usage: wgpu::BufferUsages::VERTEX | wgpu::BufferUsages::COPY_DST,
            mapped_at_creation: false,
        });

        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Bind Group"),
            layout: &bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&sampler),
                },
            ],
        });

        let mut renderer = Self {
            pipeline,
            bind_group_layout,
            bind_group,
            uniform_buffer,
            vertex_buffer,
            atlas_texture,
            atlas_view,
            sampler,
            font,
            font_size,
            ascent,
            cell_width,
            cell_height,
            glyph_cache: HashMap::new(),
            atlas_width,
            atlas_height,
            atlas_cursor_x: 0,
            atlas_cursor_y: 0,
            atlas_row_height: 0,
            vertex_data: Vec::with_capacity(300 * 300 * 6),
            max_vertices,
            width: 1024,
            height: 768,
            opacity: config.appearance.opacity,
        };

        // Pre-cache common ASCII characters
        for c in ' '..='~' {
            renderer.cache_glyph(queue, c);
        }

        Ok(renderer)
    }

    fn cache_glyph(&mut self, queue: &wgpu::Queue, c: char) {
        if self.glyph_cache.contains_key(&c) {
            return;
        }

        let (metrics, bitmap) = self.font.rasterize(c, self.font_size);

        if bitmap.is_empty() {
            // Space or non-renderable character
            self.glyph_cache.insert(
                c,
                GlyphInfo {
                    uv_min: [0.0, 0.0],
                    uv_max: [0.0, 0.0],
                    offset: [0.0, 0.0],
                    advance: metrics.advance_width,
                },
            );
            return;
        }

        let glyph_width = metrics.width as u32;
        let glyph_height = metrics.height as u32;

        // Check if we need to move to next row
        if self.atlas_cursor_x + glyph_width > self.atlas_width {
            self.atlas_cursor_x = 0;
            self.atlas_cursor_y += self.atlas_row_height + 1;
            self.atlas_row_height = 0;
        }

        // Update row height
        self.atlas_row_height = self.atlas_row_height.max(glyph_height);

        // Upload glyph to atlas
        if self.atlas_cursor_y + glyph_height <= self.atlas_height {
            queue.write_texture(
                wgpu::ImageCopyTexture {
                    texture: &self.atlas_texture,
                    mip_level: 0,
                    origin: wgpu::Origin3d {
                        x: self.atlas_cursor_x,
                        y: self.atlas_cursor_y,
                        z: 0,
                    },
                    aspect: wgpu::TextureAspect::All,
                },
                &bitmap,
                wgpu::ImageDataLayout {
                    offset: 0,
                    bytes_per_row: Some(glyph_width),
                    rows_per_image: Some(glyph_height),
                },
                wgpu::Extent3d {
                    width: glyph_width,
                    height: glyph_height,
                    depth_or_array_layers: 1,
                },
            );

            let uv_min = [
                self.atlas_cursor_x as f32 / self.atlas_width as f32,
                self.atlas_cursor_y as f32 / self.atlas_height as f32,
            ];
            let uv_max = [
                (self.atlas_cursor_x + glyph_width) as f32 / self.atlas_width as f32,
                (self.atlas_cursor_y + glyph_height) as f32 / self.atlas_height as f32,
            ];

            self.glyph_cache.insert(
                c,
                GlyphInfo {
                    uv_min,
                    uv_max,
                    offset: [metrics.xmin as f32, metrics.ymin as f32],
                    advance: metrics.advance_width,
                },
            );

            self.atlas_cursor_x += glyph_width + 1;
        }
    }

    pub fn resize(&mut self, _device: &wgpu::Device, width: u32, height: u32) {
        self.width = width;
        self.height = height;
    }

    fn get_color(&self, cell_color: CellColor, colors: &Colors, is_fg: bool) -> [f32; 4] {
        match cell_color {
            CellColor::Default => colors.foreground_rgba(),
            CellColor::DefaultBg => colors.background_rgba(),
            CellColor::Indexed(idx) => colors.get_ansi_color(idx),
            CellColor::Rgb(r, g, b) => [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0],
        }
    }

    pub fn render(
        &mut self,
        _device: &wgpu::Device,
        queue: &wgpu::Queue,
        encoder: &mut wgpu::CommandEncoder,
        output_view: &wgpu::TextureView,
        grid: &Grid,
        config: &Config,
        selection: &Selection,
        frame_count: u64,
    ) {
        self.vertex_data.clear();

        let cols = grid.cols() as usize;
        let rows = grid.rows() as usize;
        let (cursor_x, cursor_y) = grid.cursor();
        let scroll_offset = grid.scroll_offset();

        // Update uniforms
        let uniforms = TextUniforms {
            resolution: [self.width as f32, self.height as f32],
            cell_size: [self.cell_width, self.cell_height],
            atlas_size: [self.atlas_width as f32, self.atlas_height as f32],
            _padding: [0.0, 0.0],
        };
        queue.write_buffer(&self.uniform_buffer, 0, bytemuck::cast_slice(&[uniforms]));

        let scrollback = grid.scrollback();
        let selection_bg = Colors::parse_hex(&config.colors.selection);

        // Build vertex data for each visible row
        for y in 0..rows {
            for x in 0..cols {
                // Determine which cell to render based on scroll offset
                let (c, style, is_cursor_cell) = if scroll_offset > 0 && y < scroll_offset {
                    // This row shows scrollback content
                    let sb_index = scrollback.len() - scroll_offset + y;
                    if sb_index < scrollback.len() {
                        let line = &scrollback[sb_index];
                        if x < line.len() {
                            (line[x].c, line[x].style, false)
                        } else {
                            (' ', crate::terminal::CellStyle::default(), false)
                        }
                    } else {
                        (' ', crate::terminal::CellStyle::default(), false)
                    }
                } else {
                    // This row shows current grid content
                    let grid_y = y - scroll_offset.min(y);
                    if let Some(cell) = grid.get(x as u16, grid_y as u16) {
                        let is_cursor = x as u16 == cursor_x && grid_y as u16 == cursor_y
                            && grid.cursor_visible() && scroll_offset == 0;
                        (cell.c, cell.style, is_cursor)
                    } else {
                        (' ', crate::terminal::CellStyle::default(), false)
                    }
                };

                // Cache glyph if not already cached
                if !self.glyph_cache.contains_key(&c) {
                    self.cache_glyph(queue, c);
                }

                let glyph = match self.glyph_cache.get(&c) {
                    Some(g) => g,
                    None => continue,
                };

                // Calculate screen position
                let px = x as f32 * self.cell_width;
                let py = y as f32 * self.cell_height;

                // Get colors
                let mut fg = self.get_color(style.fg_color, &config.colors, true);
                let mut bg = self.get_color(style.bg_color, &config.colors, false);

                // Handle inverse video
                if style.inverse {
                    std::mem::swap(&mut fg, &mut bg);
                }

                // Handle dim
                if style.dim {
                    fg[0] *= 0.5;
                    fg[1] *= 0.5;
                    fg[2] *= 0.5;
                }

                // Handle selection highlight
                if selection.contains(x as u16, y as u16) {
                    bg = selection_bg;
                }

                // Apply opacity to background
                bg[0] *= self.opacity;
                bg[1] *= self.opacity;
                bg[2] *= self.opacity;
                bg[3] = self.opacity;

                // Handle cursor (with optional blink)
                if is_cursor_cell {
                    let cursor_visible = if config.appearance.cursor_blink {
                        // Blink at ~0.75 Hz (toggle every ~40 frames at 60fps)
                        (frame_count / 40) % 2 == 0
                    } else {
                        true
                    };
                    if cursor_visible {
                        std::mem::swap(&mut fg, &mut bg);
                    }
                }

                // Build quad vertices
                let x0 = px;
                let y0 = py;
                let x1 = px + self.cell_width;
                let y1 = py + self.cell_height;

                let [u0, v0] = glyph.uv_min;
                let [u1, v1] = glyph.uv_max;

                // Calculate glyph dimensions
                let glyph_w = (u1 - u0) * self.atlas_width as f32;
                let glyph_h = (v1 - v0) * self.atlas_height as f32;

                let baseline_y = py + self.ascent;
                let glyph_x = px + glyph.offset[0].max(0.0);
                let glyph_y = baseline_y - glyph_h - glyph.offset[1];

                // Background quad (full cell)
                self.vertex_data.extend_from_slice(&[
                    GlyphVertex { position: [x0, y0], uv: [0.0, 0.0], color: [0.0; 4], bg_color: bg },
                    GlyphVertex { position: [x1, y0], uv: [0.0, 0.0], color: [0.0; 4], bg_color: bg },
                    GlyphVertex { position: [x0, y1], uv: [0.0, 0.0], color: [0.0; 4], bg_color: bg },
                    GlyphVertex { position: [x1, y0], uv: [0.0, 0.0], color: [0.0; 4], bg_color: bg },
                    GlyphVertex { position: [x1, y1], uv: [0.0, 0.0], color: [0.0; 4], bg_color: bg },
                    GlyphVertex { position: [x0, y1], uv: [0.0, 0.0], color: [0.0; 4], bg_color: bg },
                ]);

                // Glyph quad (if character has visible glyph)
                if glyph_w > 0.0 && glyph_h > 0.0 {
                    let gx0 = glyph_x;
                    let gy0 = glyph_y;
                    let gx1 = glyph_x + glyph_w;
                    let gy1 = glyph_y + glyph_h;

                    self.vertex_data.extend_from_slice(&[
                        GlyphVertex { position: [gx0, gy0], uv: [u0, v0], color: fg, bg_color: [0.0; 4] },
                        GlyphVertex { position: [gx1, gy0], uv: [u1, v0], color: fg, bg_color: [0.0; 4] },
                        GlyphVertex { position: [gx0, gy1], uv: [u0, v1], color: fg, bg_color: [0.0; 4] },
                        GlyphVertex { position: [gx1, gy0], uv: [u1, v0], color: fg, bg_color: [0.0; 4] },
                        GlyphVertex { position: [gx1, gy1], uv: [u1, v1], color: fg, bg_color: [0.0; 4] },
                        GlyphVertex { position: [gx0, gy1], uv: [u0, v1], color: fg, bg_color: [0.0; 4] },
                    ]);
                }
            }
        }

        if self.vertex_data.is_empty() {
            return;
        }

        // Upload vertex data
        queue.write_buffer(
            &self.vertex_buffer,
            0,
            bytemuck::cast_slice(&self.vertex_data),
        );

        // Render
        let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
            label: Some("Text Render Pass"),
            color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                view: output_view,
                resolve_target: None,
                ops: wgpu::Operations {
                    load: wgpu::LoadOp::Clear(wgpu::Color {
                        r: 0.04 * self.opacity as f64,
                        g: 0.04 * self.opacity as f64,
                        b: 0.04 * self.opacity as f64,
                        a: self.opacity as f64,
                    }),
                    store: wgpu::StoreOp::Store,
                },
            })],
            depth_stencil_attachment: None,
            timestamp_writes: None,
            occlusion_query_set: None,
        });

        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_bind_group(0, &self.bind_group, &[]);
        render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
        render_pass.draw(0..self.vertex_data.len() as u32, 0..1);
    }

    pub fn set_font_size(&mut self, device: &wgpu::Device, queue: &wgpu::Queue, config: &Config) {
        let new_font_size = config.appearance.font_size;

        // Recalculate cell dimensions and ascent
        let metrics = self.font.metrics('M', new_font_size);
        self.font_size = new_font_size;
        self.cell_width = metrics.advance_width.ceil();
        self.cell_height = (new_font_size * config.appearance.line_height).ceil();

        // Update ascent for proper baseline positioning
        let line_metrics = self.font.horizontal_line_metrics(new_font_size).unwrap_or(
            fontdue::LineMetrics {
                ascent: new_font_size * 0.8,
                descent: new_font_size * -0.2,
                line_gap: 0.0,
                new_line_size: new_font_size,
            }
        );
        self.ascent = line_metrics.ascent;

        // Clear glyph cache - will be rebuilt on demand
        self.glyph_cache.clear();
        self.atlas_cursor_x = 0;
        self.atlas_cursor_y = 0;
        self.atlas_row_height = 0;

        // Clear the atlas texture
        let clear_data = vec![0u8; (self.atlas_width * self.atlas_height) as usize];
        queue.write_texture(
            wgpu::ImageCopyTexture {
                texture: &self.atlas_texture,
                mip_level: 0,
                origin: wgpu::Origin3d::ZERO,
                aspect: wgpu::TextureAspect::All,
            },
            &clear_data,
            wgpu::ImageDataLayout {
                offset: 0,
                bytes_per_row: Some(self.atlas_width),
                rows_per_image: Some(self.atlas_height),
            },
            wgpu::Extent3d {
                width: self.atlas_width,
                height: self.atlas_height,
                depth_or_array_layers: 1,
            },
        );

        // Pre-cache common ASCII characters with new font size
        for c in ' '..='~' {
            self.cache_glyph(queue, c);
        }

        // Recreate the bind group with the updated atlas
        self.bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label: Some("Text Bind Group"),
            layout: &self.bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: self.uniform_buffer.as_entire_binding(),
                },
                wgpu::BindGroupEntry {
                    binding: 1,
                    resource: wgpu::BindingResource::TextureView(&self.atlas_view),
                },
                wgpu::BindGroupEntry {
                    binding: 2,
                    resource: wgpu::BindingResource::Sampler(&self.sampler),
                },
            ],
        });

        log::info!(
            "Font size set to {}, cell: {}x{}",
            new_font_size,
            self.cell_width,
            self.cell_height
        );
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.opacity = opacity.clamp(0.2, 1.0);
    }

    pub fn cell_size(&self) -> (f32, f32) {
        (self.cell_width, self.cell_height)
    }
}
