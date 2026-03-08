use crate::config::Config;
use std::sync::Arc;
use winit::dpi::LogicalSize;
use winit::event::WindowEvent;
use winit::event_loop::EventLoopWindowTarget;
use winit::window::{Window, WindowBuilder, WindowId};

use super::tabs::SettingsAction;

pub struct SettingsWindow {
    window: Arc<Window>,
    surface: wgpu::Surface<'static>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    surface_config: wgpu::SurfaceConfiguration,
    egui_renderer: egui_wgpu::Renderer,
    egui_state: egui_winit::State,
    egui_ctx: egui::Context,
    // Settings state
    scrollback_lines_str: String,
    unlimited_scrollback: bool,
    cursor_blink: bool,
    transparency: f32,
    new_profile_name: String,
    pending_close: bool,
}

impl SettingsWindow {
    pub async fn new(
        event_loop: &EventLoopWindowTarget<()>,
        config: &Config,
    ) -> anyhow::Result<Self> {
        let window = Arc::new(
            WindowBuilder::new()
                .with_title("80sTerminal Preferences")
                .with_inner_size(LogicalSize::new(420.0, 620.0))
                .with_min_inner_size(LogicalSize::new(350.0, 400.0))
                .with_resizable(true)
                .build(event_loop)?,
        );

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(Arc::clone(&window))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::LowPower,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("No GPU adapter for settings window"))?;

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("Settings GPU"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let size = window.inner_size();
        let surface_config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &surface_config);

        let egui_ctx = egui::Context::default();
        let egui_state = egui_winit::State::new(
            egui_ctx.clone(),
            egui::ViewportId::ROOT,
            &window,
            Some(window.scale_factor() as f32),
            None,
        );

        let egui_renderer = egui_wgpu::Renderer::new(&device, surface_format, None, 1);

        // Initialize state from config
        let profile = config.active_profile_config();
        let scrollback_lines_str = profile
            .map(|p| p.scrollback_lines.to_string())
            .unwrap_or_else(|| "10000".to_string());
        let unlimited_scrollback = profile.map(|p| p.unlimited_scrollback).unwrap_or(false);

        Ok(Self {
            window,
            surface,
            device,
            queue,
            surface_config,
            egui_renderer,
            egui_state,
            egui_ctx,
            scrollback_lines_str,
            unlimited_scrollback,
            cursor_blink: config.appearance.cursor_blink,
            transparency: 1.0 - config.appearance.opacity,
            new_profile_name: String::new(),
            pending_close: false,
        })
    }

    pub fn window_id(&self) -> WindowId {
        self.window.id()
    }

    pub fn request_redraw(&self) {
        self.window.request_redraw();
    }

    pub fn should_close(&self) -> bool {
        self.pending_close
    }

    pub fn handle_event(&mut self, event: &WindowEvent) -> bool {
        let response = self.egui_state.on_window_event(&self.window, event);
        response.consumed
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        if width == 0 || height == 0 {
            return;
        }
        self.surface_config.width = width.max(1);
        self.surface_config.height = height.max(1);
        self.surface.configure(&self.device, &self.surface_config);
    }

    pub fn sync_from_config(&mut self, config: &Config) {
        self.cursor_blink = config.appearance.cursor_blink;
        self.transparency = 1.0 - config.appearance.opacity;
        if let Some(profile) = config.active_profile_config() {
            self.scrollback_lines_str = profile.scrollback_lines.to_string();
            self.unlimited_scrollback = profile.unlimited_scrollback;
        }
    }

    pub fn render(&mut self, config: &Config) -> SettingsAction {
        let mut action = SettingsAction::None;

        let output = match self.surface.get_current_texture() {
            Ok(o) => o,
            Err(_) => return action,
        };
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let raw_input = self.egui_state.take_egui_input(&self.window);

        let full_output = self.egui_ctx.run(raw_input, |ctx| {
            // Native window style
            let mut style = (*ctx.style()).clone();
            style.visuals.window_fill = egui::Color32::from_rgb(30, 30, 30);
            style.visuals.panel_fill = egui::Color32::from_rgb(30, 30, 30);
            style.visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(50, 50, 50);
            style.visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(55, 55, 55);
            style.visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(70, 70, 70);
            style.visuals.widgets.active.bg_fill = egui::Color32::from_rgb(80, 80, 80);
            style.visuals.selection.bg_fill = egui::Color32::from_rgb(50, 100, 200);
            style.visuals.override_text_color = Some(egui::Color32::from_rgb(220, 220, 220));
            style.visuals.window_rounding = egui::Rounding::same(0.0);
            style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.inactive.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.hovered.rounding = egui::Rounding::same(4.0);
            style.visuals.widgets.active.rounding = egui::Rounding::same(4.0);

            style.text_styles.insert(
                egui::TextStyle::Body,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Button,
                egui::FontId::new(14.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Heading,
                egui::FontId::new(18.0, egui::FontFamily::Proportional),
            );
            style.text_styles.insert(
                egui::TextStyle::Small,
                egui::FontId::new(12.0, egui::FontFamily::Proportional),
            );
            style.spacing.item_spacing = egui::vec2(8.0, 6.0);

            ctx.set_style(style);

            egui::CentralPanel::default()
                .frame(egui::Frame::central_panel(&ctx.style()).inner_margin(egui::Margin::symmetric(20.0, 16.0)))
                .show(ctx, |ui| {
                    egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                    ui.heading("Preferences");
                    ui.add_space(8.0);

                    // --- Appearance section ---
                    ui.label(egui::RichText::new("Appearance").strong().size(15.0));
                    ui.separator();
                    ui.add_space(4.0);

                    // Font size
                    ui.horizontal(|ui| {
                        ui.label("Font Size:");
                        ui.label(format!("{:.0} pt", config.appearance.font_size));
                        if ui.button(" - ").clicked() {
                            action = SettingsAction::FontSizeChanged(-2.0);
                        }
                        if ui.button(" + ").clicked() {
                            action = SettingsAction::FontSizeChanged(2.0);
                        }
                        if ui.button("Reset").clicked() {
                            action = SettingsAction::FontSizeChanged(0.0);
                        }
                    });

                    ui.add_space(2.0);

                    // Cursor blink
                    if ui.checkbox(&mut self.cursor_blink, "Cursor Blink").changed() {
                        action = SettingsAction::CursorBlinkChanged(self.cursor_blink);
                    }

                    ui.add_space(2.0);

                    // Window transparency
                    ui.horizontal(|ui| {
                        ui.label("Transparency:");
                        ui.label(format!("{:.0}%", self.transparency * 100.0));
                    });
                    if ui.add(
                        egui::Slider::new(&mut self.transparency, 0.0..=0.8).show_value(false)
                    ).changed() {
                        action = SettingsAction::TransparencyChanged(self.transparency);
                    }

                    ui.add_space(12.0);

                    // --- Scrollback section ---
                    ui.label(egui::RichText::new("Scrollback Buffer").strong().size(15.0));
                    ui.separator();
                    ui.add_space(4.0);

                    if ui.checkbox(&mut self.unlimited_scrollback, "Unlimited").changed() {
                        action = SettingsAction::UnlimitedScrollbackChanged(self.unlimited_scrollback);
                    }

                    if !self.unlimited_scrollback {
                        ui.horizontal(|ui| {
                            ui.label("Lines:");
                            let response = ui.add(
                                egui::TextEdit::singleline(&mut self.scrollback_lines_str)
                                    .desired_width(100.0),
                            );
                            if response.lost_focus() || ui.button("Apply").clicked() {
                                if let Ok(lines) = self.scrollback_lines_str.parse::<usize>() {
                                    let lines = lines.max(100);
                                    self.scrollback_lines_str = lines.to_string();
                                    action = SettingsAction::ScrollbackChanged(lines);
                                }
                            }
                        });
                    }

                    ui.add_space(12.0);

                    // --- Profiles section ---
                    ui.label(egui::RichText::new("Profiles").strong().size(15.0));
                    ui.separator();
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        ui.label("Active:");
                        ui.strong(&config.active_profile);
                    });

                    ui.add_space(4.0);

                    for name in config.profile_names() {
                        let is_active = name == config.active_profile;
                        ui.horizontal(|ui| {
                            let label = if is_active {
                                egui::RichText::new(&name).strong()
                            } else {
                                egui::RichText::new(&name)
                            };
                            if ui.add_enabled(!is_active, egui::Button::new(label).min_size(egui::vec2(120.0, 0.0))).clicked() {
                                action = SettingsAction::LoadProfile(name);
                            }
                        });
                    }

                    ui.add_space(6.0);

                    ui.horizontal(|ui| {
                        ui.label("Save as:");
                        ui.add(
                            egui::TextEdit::singleline(&mut self.new_profile_name)
                                .desired_width(150.0)
                                .hint_text("New profile name"),
                        );
                        if ui.button("Save").clicked() && !self.new_profile_name.is_empty() {
                            action = SettingsAction::SaveProfile(self.new_profile_name.clone());
                            self.new_profile_name.clear();
                        }
                    });

                    ui.add_space(12.0);

                    // --- Config section ---
                    ui.label(egui::RichText::new("Configuration").strong().size(15.0));
                    ui.separator();
                    ui.add_space(4.0);

                    ui.horizontal(|ui| {
                        if ui.button("Save Config to Disk").clicked() {
                            action = SettingsAction::SaveConfig;
                        }
                        if ui.button("Open Config File").clicked() {
                            action = SettingsAction::OpenConfigFile;
                        }
                    });
                    }); // ScrollArea
                });
        });

        self.egui_state
            .handle_platform_output(&self.window, full_output.platform_output);

        let tris = self
            .egui_ctx
            .tessellate(full_output.shapes, full_output.pixels_per_point);

        for (id, image_delta) in &full_output.textures_delta.set {
            self.egui_renderer
                .update_texture(&self.device, &self.queue, *id, image_delta);
        }

        let screen_descriptor = egui_wgpu::ScreenDescriptor {
            size_in_pixels: [self.surface_config.width, self.surface_config.height],
            pixels_per_point: full_output.pixels_per_point,
        };

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Settings Encoder"),
            });

        self.egui_renderer.update_buffers(
            &self.device,
            &self.queue,
            &mut encoder,
            &tris,
            &screen_descriptor,
        );

        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Settings Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.12,
                            g: 0.12,
                            b: 0.12,
                            a: 1.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                timestamp_writes: None,
                occlusion_query_set: None,
            });

            self.egui_renderer
                .render(&mut render_pass, &tris, &screen_descriptor);
        }

        for id in &full_output.textures_delta.free {
            self.egui_renderer.free_texture(id);
        }

        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();

        action
    }
}
