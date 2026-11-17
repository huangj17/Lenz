//! 屏幕位图采集(spec 02)。基于 `xcap`。
//!
//! 对外接口用逻辑坐标(相对虚拟屏左上);缩放换算在此内部完成。
//! M2 支持单屏区域;跨屏拼接见 TODO。

use std::io::Cursor;
use std::time::{Duration, Instant};

use image::{DynamicImage, ImageFormat, RgbaImage};
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
    encode_capture(shot, &plan, scale)
}

/// 编码裁剪结果为 PNG,并算出其在虚拟屏的物理像素位置(逻辑 × 当前屏 scale),回映射时用。
fn encode_capture(
    shot: RgbaImage,
    plan: &CapturePlan,
    scale: f32,
) -> Result<CaptureResult, LenzError> {
    let (pixel_w, pixel_h) = (shot.width(), shot.height());
    let mut png = Vec::new();
    DynamicImage::ImageRgba8(shot)
        .write_to(&mut Cursor::new(&mut png), ImageFormat::Png)
        .map_err(|e| LenzError::Internal(format!("png encode: {e}")))?;
    Ok(CaptureResult {
        png,
        pixel_rect: Rect {
            x: (f64::from(plan.logical_x) * f64::from(scale)).round() as i32,
            y: (f64::from(plan.logical_y) * f64::from(scale)).round() as i32,
            w: pixel_w as i32,
            h: pixel_h as i32,
        },
        scale_factor: scale,
    })
}

/// 显示器在虚拟屏中的逻辑边界 `(x, y, w, h)`。
pub type LogicalBounds = (f64, f64, f64, f64);

/// 某块显示器在触发瞬间的整屏定格图(spec 02 §2.3)。
pub struct FrozenFrame {
    pub bounds: LogicalBounds,
    pub image: RgbaImage,
}

impl FrozenFrame {
    /// 物理像素 / 逻辑点之比。按位图实际宽度计算,不依赖系统上报的 scale。
    fn scale(&self) -> f64 {
        f64::from(self.image.width()) / self.bounds.2.max(1.0)
    }
}

/// 截取全部显示器的整屏位图,带独立超时。单屏失败只记日志,全部失败才返回错误。
pub async fn freeze_all_with_timeout() -> Result<Vec<FrozenFrame>, LenzError> {
    let started = Instant::now();
    let result = tokio::time::timeout(CAPTURE_TIMEOUT, tokio::task::spawn_blocking(freeze_all))
        .await
        .map_err(|_| LenzError::Timeout("screen freeze timed out after 10s".into()))
        .and_then(|result| result.map_err(|e| LenzError::internal(format!("freeze join: {e}"))))
        .and_then(|result| result);
    match &result {
        Ok(frames) => log::info!(
            "screen frozen: monitors={} elapsed_ms={}",
            frames.len(),
            started.elapsed().as_millis()
        ),
        Err(error) => log::warn!(
            "screen freeze failed: elapsed_ms={} error={error}",
            started.elapsed().as_millis()
        ),
    }
    result
}

fn freeze_all() -> Result<Vec<FrozenFrame>, LenzError> {
    let monitors = Monitor::all().map_err(map_xcap_err)?;
    let mut frames = Vec::with_capacity(monitors.len());
    let mut first_error = None;
    for monitor in &monitors {
        match freeze_monitor(monitor) {
            Ok(frame) => frames.push(frame),
            Err(error) => {
                log::warn!("freeze monitor failed: {error}");
                first_error.get_or_insert(error);
            }
        }
    }
    if frames.is_empty() {
        return Err(first_error.unwrap_or_else(|| LenzError::Internal("no monitor found".into())));
    }
    Ok(frames)
}

fn freeze_monitor(monitor: &Monitor) -> Result<FrozenFrame, LenzError> {
    // 与 capture_region 相同:Windows 的 xcap 边界是物理像素,macOS 是逻辑点。
    let divisor = if cfg!(target_os = "windows") {
        f64::from(monitor.scale_factor().map_err(map_xcap_err)?).max(0.01)
    } else {
        1.0
    };
    let bounds = (
        f64::from(monitor.x().map_err(map_xcap_err)?) / divisor,
        f64::from(monitor.y().map_err(map_xcap_err)?) / divisor,
        f64::from(monitor.width().map_err(map_xcap_err)?) / divisor,
        f64::from(monitor.height().map_err(map_xcap_err)?) / divisor,
    );
    let image = monitor.capture_image().map_err(map_xcap_err)?;
    Ok(FrozenFrame { bounds, image })
}

/// 选出与该显示器逻辑边界对应的定格图。要求重叠超过显示器面积一半,
/// 避免显示器在定格后增减时把别的屏的画面铺上去。
pub fn frame_for_monitor(frames: &[FrozenFrame], monitor: LogicalBounds) -> Option<usize> {
    let area = monitor.2 * monitor.3;
    frames
        .iter()
        .enumerate()
        .map(|(i, frame)| (i, overlap_area(frame.bounds, monitor)))
        .filter(|(_, overlap)| *overlap > area / 2.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

/// 从定格图裁剪选区(逻辑坐标)。与选区重叠最多的定格屏胜出;与所有定格屏都不相交时
/// 返回 `None`,由调用方回退实时截屏。
pub fn crop_frozen(frames: &[FrozenFrame], rect: Rect) -> Option<Result<CaptureResult, LenzError>> {
    if rect.w < 1 || rect.h < 1 {
        return Some(Err(LenzError::Internal("capture rect too small".into())));
    }
    let selection = (
        f64::from(rect.x),
        f64::from(rect.y),
        f64::from(rect.w),
        f64::from(rect.h),
    );
    let frame = frames
        .iter()
        .map(|frame| (frame, overlap_area(frame.bounds, selection)))
        .filter(|(_, overlap)| *overlap > 0.0)
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(frame, _)| frame)?;
    Some(crop_frame(frame, rect))
}

fn crop_frame(frame: &FrozenFrame, rect: Rect) -> Result<CaptureResult, LenzError> {
    let started = Instant::now();
    let plan = frozen_crop_plan(
        rect,
        frame.bounds,
        frame.image.width(),
        frame.image.height(),
    )?;
    let scale = frame.scale() as f32;
    let shot = image::imageops::crop_imm(&frame.image, plan.x, plan.y, plan.w, plan.h).to_image();
    let result = encode_capture(shot, &plan, scale);
    if let Ok(capture) = &result {
        log::info!(
            "capture cropped from frozen frame: elapsed_ms={} pixel_size={}x{} scale={}",
            started.elapsed().as_millis(),
            capture.pixel_rect.w,
            capture.pixel_rect.h,
            capture.scale_factor
        );
    }
    result
}

/// 定格图按物理像素存储,复用实时截屏的换算(视作「物理单位」的显示器),
/// 再把四舍五入可能多出的 1px 夹回位图内。
fn frozen_crop_plan(
    rect: Rect,
    bounds: LogicalBounds,
    image_w: u32,
    image_h: u32,
) -> Result<CapturePlan, LenzError> {
    if image_w == 0 || image_h == 0 {
        return Err(LenzError::Internal("frozen frame is empty".into()));
    }
    let scale = f64::from(image_w) / bounds.2.max(1.0);
    let raw = RawMonitorBounds {
        x: bounds.0 * scale,
        y: bounds.1 * scale,
        w: f64::from(image_w),
        h: f64::from(image_h),
    };
    let mut plan = make_capture_plan(rect, raw, scale as f32, true)?;
    plan.x = plan.x.min(image_w - 1);
    plan.y = plan.y.min(image_h - 1);
    plan.w = plan.w.min(image_w - plan.x);
    plan.h = plan.h.min(image_h - plan.y);
    Ok(plan)
}

fn overlap_area(a: LogicalBounds, b: LogicalBounds) -> f64 {
    let width = (a.0 + a.2).min(b.0 + b.2) - a.0.max(b.0);
    let height = (a.1 + a.3).min(b.1 + b.3) - a.1.max(b.1);
    width.max(0.0) * height.max(0.0)
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
    use super::{
        crop_frozen, frame_for_monitor, frozen_crop_plan, make_capture_plan, CapturePlan,
        FrozenFrame, RawMonitorBounds,
    };
    use crate::types::Rect;

    fn frame(bounds: (f64, f64, f64, f64), scale: f64) -> FrozenFrame {
        FrozenFrame {
            bounds,
            image: image::RgbaImage::new(
                (bounds.2 * scale).round() as u32,
                (bounds.3 * scale).round() as u32,
            ),
        }
    }

    #[test]
    fn frozen_crop_on_retina_screen_with_negative_origin() {
        // 外接屏在主屏左上方,2x 缩放。
        let plan = frozen_crop_plan(
            Rect {
                x: -1500,
                y: -100,
                w: 300,
                h: 120,
            },
            (-1728.0, -300.0, 1728.0, 1117.0),
            3456,
            2234,
        )
        .unwrap();
        assert_eq!(
            plan,
            CapturePlan {
                x: 456,
                y: 400,
                w: 600,
                h: 240,
                logical_x: -1500,
                logical_y: -100
            }
        );
    }

    #[test]
    fn frozen_crop_on_windows_mixed_dpi_screen() {
        // 与实时截屏的 Windows 用例同一块 150% 副屏:结果应一致。
        let plan = frozen_crop_plan(
            Rect {
                x: 1400,
                y: 100,
                w: 200,
                h: 80,
            },
            (1280.0, 0.0, 1706.6666666666667, 960.0),
            2560,
            1440,
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
    fn frozen_crop_clips_to_screen_and_bitmap_edges() {
        let plan = frozen_crop_plan(
            Rect {
                x: 1400,
                y: 900,
                w: 300,
                h: 200,
            },
            (0.0, 0.0, 1512.0, 982.0),
            3024,
            1964,
        )
        .unwrap();
        assert_eq!((plan.x, plan.y, plan.w, plan.h), (2800, 1800, 224, 164));
        assert!(plan.x + plan.w <= 3024 && plan.y + plan.h <= 1964);
    }

    #[test]
    fn monitors_map_to_their_own_frozen_frames() {
        let frames = [
            frame((0.0, 0.0, 1512.0, 982.0), 2.0),
            frame((-2560.0, -171.0, 2560.0, 1440.0), 1.0),
        ];
        assert_eq!(
            frame_for_monitor(&frames, (-2560.0, -171.0, 2560.0, 1440.0)),
            Some(1)
        );
        assert_eq!(
            frame_for_monitor(&frames, (0.5, 0.0, 1512.0, 982.0)),
            Some(0)
        );
        // 定格后新接入的屏没有对应画面,该屏回退实时模式。
        assert_eq!(
            frame_for_monitor(&frames, (1512.0, 0.0, 1920.0, 1080.0)),
            None
        );
    }

    #[test]
    fn crop_uses_frame_with_largest_overlap_and_reports_physical_rect() {
        let frames = [
            frame((0.0, 0.0, 1512.0, 982.0), 2.0),
            frame((-2560.0, -171.0, 2560.0, 1440.0), 1.0),
        ];
        let capture = crop_frozen(
            &frames,
            Rect {
                x: -40,
                y: 100,
                w: 400,
                h: 300,
            },
        )
        .unwrap()
        .unwrap();
        assert_eq!(capture.scale_factor, 2.0);
        assert_eq!(
            (
                capture.pixel_rect.x,
                capture.pixel_rect.y,
                capture.pixel_rect.w,
                capture.pixel_rect.h
            ),
            (0, 200, 720, 600)
        );
        assert!(crop_frozen(
            &frames,
            Rect {
                x: 5000,
                y: 0,
                w: 100,
                h: 100
            }
        )
        .is_none());
    }

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
