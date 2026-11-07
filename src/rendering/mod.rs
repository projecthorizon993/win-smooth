//! Multi-backend rendering pipeline system.
//! Provides Direct2D, Direct3D11, and Direct3D12 backends with pluggable
//! pipeline stages for smooth rendering at 60-120fps.

pub mod pipeline;

pub use pipeline::{
    FramePipeline, FrameStats, PipelineConfig, PresentMode, RenderBackend, RenderBackendTrait,
    Renderer,
};
