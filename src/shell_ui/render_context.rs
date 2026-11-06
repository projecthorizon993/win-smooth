//! Minimal Direct2D 1.1 drawing helpers for shell UI components.
//!
//! Wraps a D2D render target and exposes: solid brushes, rounded rectangle
//! fills/strokes. Kept intentionally small — one helper per primitive the
//! taskbar needs.

use std::collections::HashMap;

use crate::shell_ui::design_tokens::Color;
use windows::Win32::Foundation::{HWND, RECT};
use windows::Win32::Graphics::Direct2D::Common::{
    D2D1_COLOR_F, D2D1_RECT_F, D2D1_ROUNDED_RECT, D2D_SIZE_U,
};
use windows::Win32::Graphics::Direct2D::{
    D2D1CreateFactory, D2D1_FACTORY_TYPE_SINGLE_THREADED, D2D1_HWND_RENDER_TARGET_PROPERTIES,
    D2D1_PRESENT_OPTIONS_NONE, D2D1_RENDER_TARGET_PROPERTIES, ID2D1Factory1, ID2D1RenderTarget1,
    ID2D1SolidColorBrush,
};
use windows::Win32::UI::HiDpi::GetDpiForWindow;
use windows::core::{Interface, Result};

/// Solid color brush cache keyed by color.
#[derive(Debug, Clone, Default)]
pub struct BrushCache {
    brushes: std::collections::HashMap<Color, ID2D1SolidColorBrush>,
}

impl BrushCache {
    pub fn get(
        &mut self,
        target: &ID2D1RenderTarget1,
        color: Color,
    ) -> Result<ID2D1SolidColorBrush> {
        if let Some(b) = self.brushes.get(&color) {
            return Ok(b.clone());
        }
        let brush = create_solid_brush(target, color)?;
        self.brushes.insert(color, brush.clone());
        Ok(brush)
    }
}

fn create_solid_brush(
    target: &ID2D1RenderTarget1,
    color: Color,
) -> Result<ID2D1SolidColorBrush> {
    unsafe {
        let d2d_color = D2D1_COLOR_F {
            r: color.r,
            g: color.g,
            b: color.b,
            a: color.a,
        };
        let brush = target.CreateSolidColorBrush(&d2d_color, None)?;
        Ok(brush)
    }
}

/// Direct2D drawing context for one frame.
pub struct DrawContext {
    pub factory: ID2D1Factory1,
    pub target: ID2D1RenderTarget1,
    pub brushes: BrushCache,
}

impl DrawContext {
    pub fn create_for_window(hwnd: HWND) -> Result<Self> {
        unsafe {
            let factory = D2D1CreateFactory::<ID2D1Factory1>(D2D1_FACTORY_TYPE_SINGLE_THREADED, None)?;

            let mut rect = RECT::default();
            windows::Win32::Graphics::Gdi::GetClientRect(hwnd, &mut rect);
            let width = (rect.right - rect.left) as u32;
            let height = (rect.bottom - rect.top) as u32;

            let target = factory.CreateHwndRenderTarget(
                &D2D1_RENDER_TARGET_PROPERTIES::default(),
                &D2D1_HWND_RENDER_TARGET_PROPERTIES {
                    hwnd,
                    pixelSize: D2D_SIZE_U { width, height },
                    presentOptions: D2D1_PRESENT_OPTIONS_NONE,
                },
            )?.cast::<ID2D1RenderTarget1>()?;

            Ok(Self {
                factory,
                target,
                brushes: BrushCache::default(),
            })
        }
    }

    /// Begin a frame: clear to transparent black.
    pub fn begin_frame(&self) {
        unsafe {
            self.target.BeginDraw();
            self.target.Clear(None);
        }
    }

    /// End a frame: present.
    pub fn end_frame(&self) -> Result<()> {
        unsafe { self.target.EndDraw(None, None) }
    }

    /// Fill a rounded rectangle with a color.
    pub fn fill_rounded_rect(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: Color,
    ) -> Result<()> {
        let brush = self.brushes.get(&self.target, color)?;
        unsafe {
            let rect = D2D1_ROUNDED_RECT {
                rect: D2D1_RECT_F {
                    left: x,
                    top: y,
                    right: x + width,
                    bottom: y + height,
                },
                radiusX: radius,
                radiusY: radius,
            };
            self.target.FillRoundedRectangle(&rect, &brush)?;
        }
        Ok(())
    }

    /// Stroke a rounded rectangle outline.
    pub fn stroke_rounded_rect(
        &self,
        x: f32,
        y: f32,
        width: f32,
        height: f32,
        radius: f32,
        color: Color,
        stroke_width: f32,
    ) -> Result<()> {
        let brush = self.brushes.get(&self.target, color)?;
        unsafe {
            let rect = D2D1_ROUNDED_RECT {
                rect: D2D1_RECT_F {
                    left: x,
                    top: y,
                    right: x + width,
                    bottom: y + height,
                },
                radiusX: radius,
                radiusY: radius,
            };
            self.target.DrawRoundedRectangle(&rect, &brush, stroke_width, None)?;
        }
        Ok(())
    }

    /// Fill a plain rectangle.
    pub fn fill_rect(&self, rect: &RECT, color: Color) -> Result<()> {
        let brush = self.brushes.get(&self.target, color)?;
        unsafe {
            self.target.FillRectangle(
                &D2D1_RECT_F {
                    left: rect.left as f32,
                    top: rect.top as f32,
                    right: rect.right as f32,
                    bottom: rect.bottom as f32,
                },
                &brush,
            )?;
        }
        Ok(())
    }

    /// Window dpi scale factor.
    pub fn dpi_scale(&self, hwnd: HWND) -> f32 {
        unsafe { GetDpiForWindow(hwnd) as f32 / 96.0 }
    }
}
