mod context;
pub mod crt;
mod text;

use crate::config::Config;
use crate::terminal::{Selection, Terminal};
use crate::ui::Ui;
use context::RenderContext;
use crt::CrtPipeline;
use std::sync::Arc;
use text::TextRenderer;
use winit::event::WindowEvent;
use winit::window::Window;

pub struct Renderer {
    context: RenderContext,
    text_renderer: TextRenderer,
    crt_pipeline: CrtPipeline,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
    frame_count: u64,
}

impl Renderer {
    pub async fn new(window: Arc<Window>, config: &Config) -> anyhow::Result<Self> {
        let context = RenderContext::new(Arc::clone(&window)).await?;

        let text_renderer = TextRenderer::new(
            &context.device,
            &context.queue,
            context.config.format,
            config,
        )?;

        let crt_pipeline = CrtPipeline::new(
            &context.device,
            context.config.format,
            context.config.width,
            context.config.height,
        );

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
        );

        let egui_renderer = egui_wgpu::Renderer::new(
            &context.device,
            context.config.format,
            None,
            1,
        );

        Ok(Self {
            context,
            text_renderer,
            crt_pipeline,
            egui_renderer,
            egui_state,
            egui_ctx,
            frame_count: 0,
        })
    }

    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        let response = self.egui_state.on_window_event(&self.context.window, event);
        response.consumed
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }

        self.context.resize(width, height);
        self.crt_pipeline.resize(
            &self.context.device,
            self.context.config.format,
            width,
            height,
        );
        self.text_renderer
            .resize(&self.context.device, width, height);
    }

    pub fn set_font_size(&mut self, config: &Config) {
        self.text_renderer.set_font_size(
            &self.context.device,
            &self.context.queue,
            config,
        );
    }

    pub fn cell_size(&self) -> (f32, f32) {
        self.text_renderer.cell_size()
    }

    pub fn set_opacity(&mut self, opacity: f32) {
        self.text_renderer.set_opacity(opacity);
    }

    pub fn render(
        &mut self,
        terminal: &Terminal,
        ui: &mut Ui,
        config: &Config,
        selection: &Selection,
    ) -> anyhow::Result<()> {
        self.frame_count += 1;

        let output = self.context.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .context
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });

        // Render terminal text to offscreen texture
        {
            let grid = terminal.grid().lock();
            self.text_renderer.render(
                &self.context.device,
                &self.context.queue,
                &mut encoder,
                self.crt_pipeline.input_view(),
                &grid,
                config,
                selection,
                self.frame_count,
            );
        }

        // Apply CRT post-processing (shift down by tab bar height)
        // Tab bar is 36 logical pixels; convert to physical pixels for the shader
        let scale_factor = self.context.window.scale_factor() as f32;
        let tab_bar_height = 36.0 * scale_factor;
        self.crt_pipeline.render(
            &self.context.queue,
            &mut encoder,
            &view,
            config,
            self.frame_count,
            tab_bar_height,
        );

        // Render egui UI on top
        let raw_input = self.egui_state.take_egui_input(&self.context.window);

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            ui.render(ctx, terminal, config);
        });

        self.egui_state.handle_platform_output(
            &self.context.window,
            full_output.platform_output,
        );

        let tris = self.egui_ctx.tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.context.device, &self.context.queue, *id, image_delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.context.config.width, self.context.config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        self.egui_renderer.update_buffers(
            &self.context.device,
            &self.context.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("egui Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Load,
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer.render(&mut render_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.context.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        Ok(())
    }
}
