//! 屏幕位图采集(spec 02)。基于 `xcap`。
//!
//! 对外接口用逻辑坐标(相对虚拟屏左上);缩放换算在此内部完成。
//! M2 支持单屏区域;跨屏拼接见 TODO。

use std::io::Cursor;

use image::{DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};
use specta::Type;
use xcap::Monitor;

use crate::error::LenzError;
use crate::types::Rect;

/// 采集结果(spec 02 §4)。`png` 为裁剪区域;`pixel_rect` 为其在虚拟屏的物理像素位置(供 OCR 回映射)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub png: Vec<u8>,
    pub pixel_rect: Rect,
    pub scale_factor: f32,
}

/// 采集指定逻辑区域(相对虚拟屏左上)。
// TODO(M2): 跨屏选区的物理像素拼接;当前取起点所在单屏。
pub fn capture_region(rect: Rect) -> Result<CaptureResult, LenzError> {
    if rect.w < 1 || rect.h < 1 {
        return Err(LenzError::Internal("capture rect too small".into()));
    }
    let monitors = Monitor::all().map_err(map_xcap_err)?;
    // 找选区起点所在的显示器(xcap 的几何访问器均返回 Result)。
    let mut chosen: Option<(&Monitor, i32, i32, f32)> = None;
    for m in &monitors {
        let mx = m.x().map_err(map_xcap_err)?;
        let my = m.y().map_err(map_xcap_err)?;
        let mw = m.width().map_err(map_xcap_err)? as i32;
        let mh = m.height().map_err(map_xcap_err)? as i32;
        if rect.x >= mx && rect.x < mx + mw && rect.y >= my && rect.y < my + mh {
            chosen = Some((m, mx, my, m.scale_factor().map_err(map_xcap_err)?));
            break;
        }
    }
    // 回退到首个显示器。
    let (monitor, mon_x, mon_y, scale) = match chosen {
        Some(t) => t,
        None => {
            let m = monitors
                .first()
                .ok_or_else(|| LenzError::Internal("no monitor found".into()))?;
            (
                m,
                m.x().map_err(map_xcap_err)?,
                m.y().map_err(map_xcap_err)?,
                m.scale_factor().map_err(map_xcap_err)?,
            )
        }
    };

    // 整屏物理像素位图。
    let shot = monitor.capture_image().map_err(map_xcap_err)?;
    let (img_w, img_h) = (shot.width(), shot.height());

    // 逻辑坐标 → 该屏内物理像素,并夹紧到屏幕边界避免越界。
    let local_x = (((rect.x - mon_x) as f32) * scale).round().max(0.0) as u32;
    let local_y = (((rect.y - mon_y) as f32) * scale).round().max(0.0) as u32;
    let crop_w = (((rect.w as f32) * scale).round() as u32)
        .min(img_w.saturating_sub(local_x))
        .max(1);
    let crop_h = (((rect.h as f32) * scale).round() as u32)
        .min(img_h.saturating_sub(local_y))
        .max(1);

    let cropped =
        image::imageops::crop_imm(&shot, local_x, local_y, crop_w, crop_h).to_image();

    let mut png = Vec::new();
    DynamicImage::ImageRgba8(cropped)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| LenzError::Internal(format!("png encode: {e}")))?;

    // 截图区域在虚拟屏的物理像素(逻辑 × scale),回映射时用。
    let pixel_rect = Rect {
        x: ((rect.x as f32) * scale).round() as i32,
        y: ((rect.y as f32) * scale).round() as i32,
        w: crop_w as i32,
        h: crop_h as i32,
    };

    Ok(CaptureResult {
        png,
        pixel_rect,
        scale_factor: scale,
    })
}

/// xcap 错误映射:macOS 未授予「屏幕录制」时采集失败 → Permission。
fn map_xcap_err(e: xcap::XCapError) -> LenzError {
    let msg = e.to_string();
    let low = msg.to_lowercase();
    if low.contains("permission") || low.contains("denied") || low.contains("authoriz") {
        LenzError::Permission(format!("screen capture: {msg}"))
    } else {
        LenzError::Internal(format!("capture: {msg}"))
    }
}
