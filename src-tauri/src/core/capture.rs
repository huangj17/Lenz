//! 屏幕位图采集(spec 02)。基于 `xcap`。
//!
//! 对外接口用逻辑坐标(相对虚拟屏左上);缩放换算在此内部完成。
//! M2 支持单屏区域;跨屏拼接见 TODO。

use std::io::Cursor;
use std::time::{Duration, Instant};

use image::{DynamicImage, ImageFormat};
use serde::{Deserialize, Serialize};
use specta::Type;
use xcap::Monitor;

use crate::error::LenzError;
use crate::types::Rect;

const CAPTURE_TIMEOUT: Duration = Duration::from_secs(10);

/// 截图独立计时,不消耗 OCR 的初始化预算。日志只记录尺寸和耗时。
pub async fn capture_region_with_timeout(rect: Rect) -> Result<CaptureResult, LenzError> {
    let started = Instant::now();
    log::info!("capture started: logical_size={}x{}", rect.w, rect.h);
    let result = tokio::time::timeout(
        CAPTURE_TIMEOUT,
        tokio::task::spawn_blocking(move || capture_region(rect)),
    )
    .await
    .map_err(|_| LenzError::Timeout("capture timed out after 10s".into()))
    .and_then(|result| result.map_err(|e| LenzError::internal(format!("capture join: {e}"))))
    .and_then(|result| result);
    match &result {
        Ok(capture) => log::info!(
            "capture finished: elapsed_ms={} pixel_size={}x{} scale={}",
            started.elapsed().as_millis(),
            capture.pixel_rect.w,
            capture.pixel_rect.h,
            capture.scale_factor
        ),
        Err(error) => log::warn!(
            "capture failed: elapsed_ms={} error={error}",
            started.elapsed().as_millis()
        ),
    }
    result
}

/// 采集结果(spec 02 §4)。`png` 为裁剪区域;`pixel_rect` 为其在虚拟屏的物理像素位置(供 OCR 回映射)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct CaptureResult {
    pub png: Vec<u8>,
    pub pixel_rect: Rect,
    pub scale_factor: f32,
}

/// 采集指定逻辑区域(相对虚拟屏左上)。
///
/// 多屏:选择**与选区重叠面积最大**的显示器(而非仅起点所在),更稳健地区分
/// 选区主要落在哪块屏。只请求 xcap 采集相交区域,避免先分配整块 4K/5K 位图。
// TODO(M2): 跨屏选区的物理像素拼接;当前取重叠最大的单屏。
pub fn capture_region(rect: Rect) -> Result<CaptureResult, LenzError> {
    if rect.w < 1 || rect.h < 1 {
        return Err(LenzError::Internal("capture rect too small".into()));
    }
    let monitors = Monitor::all().map_err(map_xcap_err)?;
    let monitor = pick_monitor(&monitors, rect)?;
    let scale = monitor.scale_factor().map_err(map_xcap_err)?;
    let raw_bounds = RawMonitorBounds {
        x: monitor.x().map_err(map_xcap_err)? as f64,
        y: monitor.y().map_err(map_xcap_err)? as f64,
        w: monitor.width().map_err(map_xcap_err)? as f64,
        h: monitor.height().map_err(map_xcap_err)? as f64,
    };
    let plan = make_capture_plan(rect, raw_bounds, scale, cfg!(target_os = "windows"))?;
    log::debug!(
        "capture_region rect=({},{},{},{}) -> local=({},{},{},{}) scale={}",
        rect.x,
        rect.y,
        rect.w,
        rect.h,
        plan.x,
        plan.y,
        plan.w,
        plan.h,
        scale
    );

    // Windows 的 xcap 坐标/尺寸是物理像素;macOS 是 CoreGraphics 逻辑点。
    // make_capture_plan 已按平台换算为 capture_region 期望的屏内单位。
    let shot = monitor
        .capture_region(plan.x, plan.y, plan.w, plan.h)
        .map_err(map_xcap_err)?;
    let (pixel_w, pixel_h) = (shot.width(), shot.height());

    let mut png = Vec::new();
    DynamicImage::ImageRgba8(shot)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| LenzError::Internal(format!("png encode: {e}")))?;

    // 截图区域在虚拟屏的物理像素(逻辑 × 当前屏 scale),回映射时用。
    let pixel_rect = Rect {
        x: (f64::from(plan.logical_x) * f64::from(scale)).round() as i32,
        y: (f64::from(plan.logical_y) * f64::from(scale)).round() as i32,
        w: pixel_w as i32,
        h: pixel_h as i32,
    };

    Ok(CaptureResult {
        png,
        pixel_rect,
        scale_factor: scale,
    })
}

#[derive(Debug, Clone, Copy)]
struct RawMonitorBounds {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct CapturePlan {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    logical_x: i32,
    logical_y: i32,
}

/// 把虚拟屏逻辑选区换成 xcap 的屏内采集单位。Windows 的 monitor 元数据与
/// capture_region 均为物理像素;macOS 的元数据/入参均为逻辑点。
fn make_capture_plan(
    rect: Rect,
    raw: RawMonitorBounds,
    scale: f32,
    raw_is_physical: bool,
) -> Result<CapturePlan, LenzError> {
    let scale = f64::from(scale.max(0.01));
    let divisor = if raw_is_physical { scale } else { 1.0 };
    let unit_scale = if raw_is_physical { scale } else { 1.0 };
    let logical = RawMonitorBounds {
        x: raw.x / divisor,
        y: raw.y / divisor,
        w: raw.w / divisor,
        h: raw.h / divisor,
    };

    let left = f64::from(rect.x).max(logical.x);
    let top = f64::from(rect.y).max(logical.y);
    let right = f64::from(rect.x + rect.w).min(logical.x + logical.w);
    let bottom = f64::from(rect.y + rect.h).min(logical.y + logical.h);
    if right <= left || bottom <= top {
        return Err(LenzError::Internal(
            "capture rect is outside monitor".into(),
        ));
    }

    Ok(CapturePlan {
        x: ((left - logical.x) * unit_scale).round().max(0.0) as u32,
        y: ((top - logical.y) * unit_scale).round().max(0.0) as u32,
        w: ((right - left) * unit_scale).round().max(1.0) as u32,
        h: ((bottom - top) * unit_scale).round().max(1.0) as u32,
        logical_x: left.round() as i32,
        logical_y: top.round() as i32,
    })
}

/// 在所有显示器中选出与 `rect`(逻辑坐标)重叠面积最大者;无重叠则回退首屏。
fn pick_monitor(monitors: &[Monitor], rect: Rect) -> Result<&Monitor, LenzError> {
    let mut best: Option<(&Monitor, i64)> = None;
    for m in monitors {
        let scale = m.scale_factor().map_err(map_xcap_err)?;
        let divisor = if cfg!(target_os = "windows") {
            scale
        } else {
            1.0
        };
        let mx = (m.x().map_err(map_xcap_err)? as f32 / divisor).round() as i32;
        let my = (m.y().map_err(map_xcap_err)? as f32 / divisor).round() as i32;
        let mw = (m.width().map_err(map_xcap_err)? as f32 / divisor).round() as i32;
        let mh = (m.height().map_err(map_xcap_err)? as f32 / divisor).round() as i32;
        // 选区与该屏的交集面积。
        let ix = (rect.x.max(mx)).min(rect.x + rect.w);
        let iy = (rect.y.max(my)).min(rect.y + rect.h);
        let iw = ((rect.x + rect.w).min(mx + mw) - ix).max(0) as i64;
        let ih = ((rect.y + rect.h).min(my + mh) - iy).max(0) as i64;
        let area = iw * ih;
        if area > 0 && best.is_none_or(|(_, b)| area > b) {
            best = Some((m, area));
        }
    }
    match best {
        Some((m, _)) => Ok(m),
        // 选区完全在所有屏之外(异常):回退首屏。
        None => monitors
            .first()
            .ok_or_else(|| LenzError::Internal("no monitor found".into())),
    }
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

#[cfg(test)]
mod tests {
    use super::{make_capture_plan, CapturePlan, RawMonitorBounds};
    use crate::types::Rect;

    #[test]
    fn windows_mixed_dpi_uses_logical_monitor_bounds_and_physical_capture_units() {
        let plan = make_capture_plan(
            Rect {
                x: 1400,
                y: 100,
                w: 200,
                h: 80,
            },
            RawMonitorBounds {
                x: 1920.0,
                y: 0.0,
                w: 2560.0,
                h: 1440.0,
            },
            1.5,
            true,
        )
        .unwrap();
        assert_eq!(
            plan,
            CapturePlan {
                x: 180,
                y: 150,
                w: 300,
                h: 120,
                logical_x: 1400,
                logical_y: 100
            }
        );
    }

    #[test]
    fn capture_plan_clips_selection_to_monitor_intersection() {
        let plan = make_capture_plan(
            Rect {
                x: -20,
                y: 10,
                w: 100,
                h: 50,
            },
            RawMonitorBounds {
                x: 0.0,
                y: 0.0,
                w: 1920.0,
                h: 1080.0,
            },
            2.0,
            false,
        )
        .unwrap();
        assert_eq!(
            plan,
            CapturePlan {
                x: 0,
                y: 10,
                w: 80,
                h: 50,
                logical_x: 0,
                logical_y: 10
            }
        );
    }
}
