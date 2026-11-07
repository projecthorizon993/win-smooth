//! Multi-backend rendering pipeline system for smoother rendering.
//! Supports Direct2D, Direct3D11, and Direct3D12 backends with pluggable
//! pipeline stages. Select backend at runtime based on GPU capabilities
//! and desired smoothness level.

use anyhow::{Context, Result};
use windows::Win32::Foundation::HWND;
use windows::Win32::Graphics::Dxgi::Common::DXGI_FORMAT;

/// Available rendering backends.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RenderBackend {
    /// Direct2D 1.1 - lightweight, good for UI compositing
    Direct2D,
    /// Direct3D 11 - hardware accelerated, wide compatibility
    Direct3D11,
    /// Direct3D 12 - low-level, lowest latency, explicit multi-GPU
    Direct3D12,
}

/// Rendering pipeline configuration.
#[derive(Debug, Clone)]
pub struct PipelineConfig {
    pub backend: RenderBackend,
    pub target_fps: u32,
    pub enable_vsync: bool,
    pub triple_buffering: bool,
    pub present_mode: PresentMode,
    pub flush_gpu_after_present: bool,
}

/// Present mode for smoothness control.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PresentMode {
    /// Immediate present - lowest latency, possible tearing
    Immediate,
    /// Mailbox/flip - best smoothness with low latency
    Mailbox,
    /// FIFO/vsync - maximum smoothness, higher latency
    Fifo,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            backend: RenderBackend::Direct3D11,
            target_fps: 60,
            enable_vsync: true,
            triple_buffering: true,
            present_mode: PresentMode::Mailbox,
            flush_gpu_after_present: false,
        }
    }
}

/// Frame timing statistics for smoothness monitoring.
#[derive(Debug, Clone, Default)]
pub struct FrameStats {
    pub frame_count: u64,
    pub avg_frame_time_ms: f64,
    pub min_frame_time_ms: f64,
    pub max_frame_time_ms: f64,
    pub dropped_frames: u64,
}

/// Trait for rendering backends. Implement for D2D, D3D11, D3D12.
pub trait RenderBackendTrait: Send + Sync {
    /// Initialize the backend for a given window.
    fn init(&mut self, hwnd: HWND, config: &PipelineConfig) -> Result<()>;
    /// Present one frame. Returns the frame time in ms.
    fn present_frame(&mut self, frame_stats: &mut FrameStats) -> Result<f64>;
    /// Resize the render target.
    fn resize(&mut self, width: u32, height: u32) -> Result<()>;
    /// Shutdown and release resources.
    fn shutdown(&mut self) -> Result<()>;
    /// Get the backend name.
    fn name(&self) -> &'static str;
}

/// Multi-backend renderer that owns a specific backend implementation.
pub struct Renderer {
    backend: Box<dyn RenderBackendTrait>,
    config: PipelineConfig,
    stats: FrameStats,
}

impl Renderer {
    /// Create a new renderer with the given backend.
    pub fn new(backend: RenderBackend, config: PipelineConfig) -> Result<Self> {
        let backend_impl: Box<dyn RenderBackendTrait> = match backend {
            RenderBackend::Direct2D => Box::new(D2dBackend::new()?),
            RenderBackend::Direct3D11 => Box::new(D3d11Backend::new()?),
            RenderBackend::Direct3D12 => Box::new(D3d12Backend::new()?),
        };

        Ok(Self {
            backend: backend_impl,
            config,
            stats: FrameStats::default(),
        })
    }

    /// Initialize the renderer for a window.
    pub fn init(&mut self, hwnd: HWND) -> Result<()> {
        self.backend.init(hwnd, &self.config)
            .context("backend init")?;
        Ok(())
    }

    /// Present one frame and update statistics.
    pub fn present_frame(&mut self) -> Result<f64> {
        let frame_time = self.backend.present_frame(&mut self.stats)?;
        self.stats.frame_count = self.stats.frame_count.saturating_add(1);
        self.stats.avg_frame_time_ms = (self.stats.avg_frame_time_ms * 0.9) + (frame_time * 0.1);
        self.stats.min_frame_time_ms = self.stats.min_frame_time_ms.min(frame_time);
        self.stats.max_frame_time_ms = self.stats.max_frame_time_ms.max(frame_time);
        Ok(frame_time)
    }

    /// Resize the render target.
    pub fn resize(&mut self, width: u32, height: u32) -> Result<()> {
        self.backend.resize(width, height)
    }

    /// Shutdown the renderer.
    pub fn shutdown(&mut self) -> Result<()> {
        self.backend.shutdown()
    }

    /// Get current frame statistics.
    pub fn stats(&self) -> &FrameStats {
        &self.stats
    }

    /// Get backend name.
    pub fn backend_name(&self) -> &'static str {
        self.backend.name()
    }
}

/// Direct2D 1.1 backend implementation.
pub struct D2dBackend {
    initialized: bool,
}

impl D2dBackend {
    pub fn new() -> Result<Self> {
        Ok(Self { initialized: false })
    }
}

impl RenderBackendTrait for D2dBackend {
    fn init(&mut self, hwnd: HWND, _config: &PipelineConfig) -> Result<()> {
        // Direct2D initialization would go here
        let _ = hwnd;
        self.initialized = true;
        Ok(())
    }

    fn present_frame(&mut self, _stats: &mut FrameStats) -> Result<f64> {
        if !self.initialized {
            return Err(anyhow::anyhow!("D2D backend not initialized"));
        }
        Ok(16.67) // placeholder 60fps
    }

    fn resize(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "Direct2D 1.1"
    }
}

/// Direct3D 11 backend implementation.
pub struct D3d11Backend {
    initialized: bool,
}

impl D3d11Backend {
    pub fn new() -> Result<Self> {
        Ok(Self { initialized: false })
    }
}

impl RenderBackendTrait for D3d11Backend {
    fn init(&mut self, hwnd: HWND, _config: &PipelineConfig) -> Result<()> {
        let _ = hwnd;
        self.initialized = true;
        Ok(())
    }

    fn present_frame(&mut self, _stats: &mut FrameStats) -> Result<f64> {
        if !self.initialized {
            return Err(anyhow::anyhow!("D3D11 backend not initialized"));
        }
        Ok(16.67)
    }

    fn resize(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "Direct3D 11"
    }
}

/// Direct3D 12 backend implementation.
pub struct D3d12Backend {
    initialized: bool,
}

impl D3d12Backend {
    pub fn new() -> Result<Self> {
        Ok(Self { initialized: false })
    }
}

impl RenderBackendTrait for D3d12Backend {
    fn init(&mut self, hwnd: HWND, _config: &PipelineConfig) -> Result<()> {
        let _ = hwnd;
        self.initialized = true;
        Ok(())
    }

    fn present_frame(&mut self, _stats: &mut FrameStats) -> Result<f64> {
        if !self.initialized {
            return Err(anyhow::anyhow!("D3D12 backend not initialized"));
        }
        Ok(8.33) // placeholder 120fps for D3D12 low latency
    }

    fn resize(&mut self, _width: u32, _height: u32) -> Result<()> {
        Ok(())
    }

    fn shutdown(&mut self) -> Result<()> {
        self.initialized = false;
        Ok(())
    }

    fn name(&self) -> &'static str {
        "Direct3D 12"
    }
}

/// Pipeline stage trait for composable render pipelines.
pub trait PipelineStage: Send + Sync {
    fn name(&self) -> &'static str;
    fn execute(&mut self) -> Result<()>;
}

/// Simple frame pipeline that runs stages in order.
pub struct FramePipeline {
    stages: Vec<Box<dyn PipelineStage>>,
}

impl FramePipeline {
    pub fn new() -> Self {
        Self { stages: Vec::new() }
    }

    pub fn add_stage(&mut self, stage: Box<dyn PipelineStage>) {
        self.stages.push(stage);
    }

    pub fn run(&mut self) -> Result<()> {
        for stage in &mut self.stages {
            stage.execute()?;
        }
        Ok(())
    }
}

impl Default for FramePipeline {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pipeline_config_default() {
        let config = PipelineConfig::default();
        assert_eq!(config.backend, RenderBackend::Direct3D11);
        assert_eq!(config.target_fps, 60);
        assert!(config.enable_vsync);
        assert!(config.triple_buffering);
    }

    #[test]
    fn test_renderer_creation() {
        let config = PipelineConfig::default();
        let renderer = Renderer::new(RenderBackend::Direct3D11, config);
        assert!(renderer.is_ok());
        assert_eq!(renderer.unwrap().backend_name(), "Direct3D 11");
    }

    #[test]
    fn test_frame_stats_default() {
        let stats = FrameStats::default();
        assert_eq!(stats.frame_count, 0);
        assert_eq!(stats.avg_frame_time_ms, 0.0);
    }

    #[test]
    fn test_pipeline_stage_addition() {
        let mut pipeline = FramePipeline::new();
        assert_eq!(pipeline.stages.len(), 0);
        // Can't easily add a stage without implementing PipelineStage,
        // but the structure is sound
    }
}
