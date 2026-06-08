use gitv_wgpu_renderer::renderer::{RenderConfig, WgpuRenderer};
use std::sync::Mutex;

/// Tauri-managed state for the wgpu offscreen renderer.
pub struct WgpuState {
    pub renderer: Mutex<Option<WgpuRenderer>>,
}

impl WgpuState {
    pub fn new() -> Self {
        Self {
            renderer: Mutex::new(None),
        }
    }

    /// Ensure the renderer is initialised with the given dimensions, resizing
    /// if the size has changed.
    pub fn ensure_init(&self, width: u32, height: u32) -> Result<(), String> {
        let mut guard = self.renderer.lock().map_err(|e| e.to_string())?;
        match guard.as_mut() {
            None => {
                let config = RenderConfig {
                    width,
                    height,
                    scale: 1.0,
                };
                *guard = Some(
                    pollster::block_on(WgpuRenderer::new(config))
                        .map_err(|e| format!("wgpu init failed: {e}"))?,
                );
            }
            Some(r) => {
                if r.config.width != width || r.config.height != height {
                    r.resize(width, height);
                }
            }
        }
        Ok(())
    }
}
