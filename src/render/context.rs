use log::info;
use std::sync::Arc;
use winit::window::Window;

pub struct RenderContext {
    pub window: Arc<Window>,
    pub surface: wgpu::Surface<'static>,
    pub device: wgpu::Device,
    pub queue: wgpu::Queue,
    pub config: wgpu::SurfaceConfiguration,
}

impl RenderContext {
    pub async fn new(window: Arc<Window>) -> anyhow::Result<Self> {
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::all(),
            ..Default::default()
        });

        let surface = instance.create_surface(Arc::clone(&window))?;

        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::HighPerformance,
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .await
            .ok_or_else(|| anyhow::anyhow!("Failed to find suitable GPU adapter"))?;

        info!("Using GPU: {:?}", adapter.get_info().name);

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    label: Some("GPU Device"),
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                },
                None,
            )
            .await?;

        let surface_caps = surface.get_capabilities(&adapter);

        info!("Available alpha modes: {:?}", surface_caps.alpha_modes);

        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        // Prefer PostMultiplied for macOS window transparency, then PreMultiplied
        let alpha_mode = if surface_caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PostMultiplied) {
            info!("Using PostMultiplied alpha mode");
            wgpu::CompositeAlphaMode::PostMultiplied
        } else if surface_caps.alpha_modes.contains(&wgpu::CompositeAlphaMode::PreMultiplied) {
            info!("Using PreMultiplied alpha mode");
            wgpu::CompositeAlphaMode::PreMultiplied
        } else {
            info!("Using default alpha mode: {:?}", surface_caps.alpha_modes[0]);
            surface_caps.alpha_modes[0]
        };

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width.max(1),
            height: size.height.max(1),
            present_mode: wgpu::PresentMode::AutoVsync,
            alpha_mode,
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };
        surface.configure(&device, &config);

        // macOS: ensure the window and its backing layer support transparency
        #[cfg(target_os = "macos")]
        {
            use cocoa::appkit::NSWindow;
            use cocoa::base::id;
            use objc::msg_send;
            use objc::sel;
            use objc::sel_impl;
            use raw_window_handle::HasWindowHandle;

            if let Ok(handle) = window.window_handle() {
                if let raw_window_handle::RawWindowHandle::AppKit(appkit_handle) = handle.as_raw() {
                    let ns_view: id = appkit_handle.ns_view.as_ptr() as id;
                    let ns_window: id = unsafe { msg_send![ns_view, window] };
                    if !ns_window.is_null() {
                        unsafe {
                            ns_window.setOpaque_(cocoa::base::NO);
                            ns_window.setBackgroundColor_(
                                cocoa::appkit::NSColor::clearColor(cocoa::base::nil),
                            );
                        }
                        info!("macOS window configured for transparency");
                    }
                }
            }
        }

        Ok(Self {
            window,
            surface,
            device,
            queue,
            config,
        })
    }

    pub fn resize(&mut self, width: u32, height: u32) {
        self.config.width = width.max(1);
        self.config.height = height.max(1);
        self.surface.configure(&self.device, &self.config);
    }
}
