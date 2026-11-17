//! 截图定格(spec 02 §2.3)。
//!
//! 按下截图 / 覆盖快捷键的瞬间先截取全部显示器,选区窗口以定格图为背景,框选完成后
//! 从内存裁剪。蒙层抢焦点、遮住鼠标都不会再让 tooltip、菜单、下拉等易失内容丢失。
//!
//! 定格图只驻留内存、不落盘:选区完成时被取走,取消或新一次触发时丢弃。选区窗口通过
//! `lenz-frame` 协议读取 BMP(仅追加文件头,免去整屏 PNG 编码的数百毫秒)。

use std::sync::{Arc, Mutex};
use std::time::Duration;

use image::RgbaImage;
use tauri::http::{header, Request, Response, StatusCode};
use tauri::{AppHandle, Manager};

use crate::core::capture::{self, CaptureResult, FrozenFrame};
use crate::error::LenzError;
use crate::types::Rect;

/// 选区窗口读取定格图的自定义协议名。
pub const FRAME_SCHEME: &str = "lenz-frame";

/// 无定格图时回退实时截屏前,等待选区蒙层从屏幕消失:覆盖「事件线程→主线程」派发 +
/// 合成器出帧,避免截到遮罩 / 销毁残帧导致 OCR 空结果(spec 02 §7)。
const LIVE_CAPTURE_DELAY: Duration = Duration::from_millis(150);

/// 一次触发的全部显示器定格图;`token` 区分不同触发,旧选区窗口的请求拿不到新图。
pub struct FrozenScreen {
    pub token: u64,
    pub frames: Vec<FrozenFrame>,
}

#[derive(Default)]
struct Inner {
    generation: u64,
    screen: Option<Arc<FrozenScreen>>,
}

/// 当前定格会话。作为 managed state 注入。
#[derive(Default)]
pub struct FreezeState(Mutex<Inner>);

impl FreezeState {
    fn lock(&self) -> std::sync::MutexGuard<'_, Inner> {
        self.0.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// 开始新一次触发:丢弃旧定格图,并让仍在截屏的旧触发作废。
    pub fn begin(&self) -> u64 {
        let mut inner = self.lock();
        inner.generation += 1;
        inner.screen = None;
        inner.generation
    }

    pub fn is_current(&self, token: u64) -> bool {
        self.lock().generation == token
    }

    /// 保存定格图;截屏期间已有新触发则丢弃并返回 `None`。
    pub fn store(&self, token: u64, frames: Vec<FrozenFrame>) -> Option<Arc<FrozenScreen>> {
        let mut inner = self.lock();
        if inner.generation != token {
            return None;
        }
        let screen = Arc::new(FrozenScreen { token, frames });
        inner.screen = Some(screen.clone());
        Some(screen)
    }

    /// 选区完成时取走定格图,之后协议不再提供这些画面。
    pub fn take(&self) -> Option<Arc<FrozenScreen>> {
        self.lock().screen.take()
    }

    pub fn clear(&self) {
        self.lock().screen = None;
    }

    fn get(&self, token: u64) -> Option<Arc<FrozenScreen>> {
        self.lock()
            .screen
            .as_ref()
            .filter(|screen| screen.token == token)
            .cloned()
    }
}

/// 协议路径中的帧标识:`{token}-{index}`。
pub fn frame_key(token: u64, index: usize) -> String {
    format!("{token}-{index}")
}

fn parse_frame_key(key: &str) -> Option<(u64, usize)> {
    let (token, index) = key.split_once('-')?;
    Some((token.parse().ok()?, index.parse().ok()?))
}

/// 处理 `lenz-frame://localhost/{token}-{index}`:返回该屏定格图(BMP)。
/// 会话已结束或标识不匹配时返回 404。
pub fn serve(app: &AppHandle, request: &Request<Vec<u8>>) -> Response<Vec<u8>> {
    let key = request.uri().path().trim_start_matches('/');
    let body = parse_frame_key(key).and_then(|(token, index)| {
        let screen = app.state::<FreezeState>().get(token)?;
        screen
            .frames
            .get(index)
            .map(|frame| encode_bmp(&frame.image))
    });
    let response = match body {
        Some(body) => Response::builder()
            .header(header::CONTENT_TYPE, "image/bmp")
            .header(header::CACHE_CONTROL, "no-store")
            .body(body),
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(Vec::new()),
    };
    response.unwrap_or_else(|_| Response::new(Vec::new()))
}

/// 把 RGBA 位图包装为 32 位 BMP:BITMAPV4HEADER + BI_BITFIELDS 声明 RGBA 字节序,
/// 负高度表示自上而下的行序,像素区可原样拷贝。不声明 alpha 通道,整图按不透明解码。
fn encode_bmp(image: &RgbaImage) -> Vec<u8> {
    const FILE_HEADER: u32 = 14;
    const INFO_HEADER: u32 = 108;
    const BI_BITFIELDS: u32 = 3;
    const LCS_SRGB: u32 = 0x7352_4742;
    const PIXELS_PER_METER: i32 = 2835;

    let pixels = image.as_raw();
    let offset = FILE_HEADER + INFO_HEADER;
    let mut out = Vec::with_capacity(offset as usize + pixels.len());
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(offset + pixels.len() as u32).to_le_bytes());
    out.extend_from_slice(&[0; 4]);
    out.extend_from_slice(&offset.to_le_bytes());

    out.extend_from_slice(&INFO_HEADER.to_le_bytes());
    out.extend_from_slice(&(image.width() as i32).to_le_bytes());
    out.extend_from_slice(&(-(image.height() as i32)).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&32u16.to_le_bytes());
    out.extend_from_slice(&BI_BITFIELDS.to_le_bytes());
    out.extend_from_slice(&(pixels.len() as u32).to_le_bytes());
    out.extend_from_slice(&PIXELS_PER_METER.to_le_bytes());
    out.extend_from_slice(&PIXELS_PER_METER.to_le_bytes());
    out.extend_from_slice(&[0; 8]);
    for mask in [0x0000_00ffu32, 0x0000_ff00, 0x00ff_0000, 0] {
        out.extend_from_slice(&mask.to_le_bytes());
    }
    out.extend_from_slice(&LCS_SRGB.to_le_bytes());
    out.extend_from_slice(&[0; 48]);
    debug_assert_eq!(out.len(), offset as usize);
    out.extend_from_slice(pixels);
    out
}

/// 选区截图:优先从定格图裁剪;没有可用定格图(定格失败,或选区落在定格后新接入的屏)
/// 时等蒙层离屏后实时截屏。
pub async fn capture_selection(
    frozen: Option<Arc<FrozenScreen>>,
    region: Rect,
) -> Result<CaptureResult, LenzError> {
    if let Some(screen) = frozen {
        let cropped =
            tokio::task::spawn_blocking(move || capture::crop_frozen(&screen.frames, region))
                .await
                .map_err(|e| LenzError::internal(format!("crop join: {e}")))?;
        if let Some(result) = cropped {
            return result;
        }
        log::warn!("selection is outside the frozen monitors; capturing live");
    }
    tokio::time::sleep(LIVE_CAPTURE_DELAY).await;
    capture::capture_region_with_timeout(region).await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bmp_round_trips_rgba_pixels_as_opaque_top_down_image() {
        let mut image = RgbaImage::new(3, 2);
        image.put_pixel(0, 0, image::Rgba([255, 0, 0, 0]));
        image.put_pixel(2, 1, image::Rgba([10, 20, 30, 255]));
        let bmp = encode_bmp(&image);
        assert_eq!(&bmp[..2], b"BM");
        assert_eq!(
            u32::from_le_bytes(bmp[2..6].try_into().unwrap()) as usize,
            bmp.len()
        );
        let decoded = image::load_from_memory_with_format(&bmp, image::ImageFormat::Bmp)
            .unwrap()
            .to_rgba8();
        assert_eq!(decoded.dimensions(), (3, 2));
        // 源数据 alpha 为 0 也按不透明显示,首行在上。
        assert_eq!(decoded.get_pixel(0, 0).0[..3], [255, 0, 0]);
        assert_eq!(decoded.get_pixel(2, 1).0[..3], [10, 20, 30]);
    }

    #[test]
    fn frame_keys_round_trip_and_reject_garbage() {
        assert_eq!(parse_frame_key(&frame_key(42, 1)), Some((42, 1)));
        assert_eq!(parse_frame_key("42"), None);
        assert_eq!(parse_frame_key("x-1"), None);
        assert_eq!(parse_frame_key("1-%2E%2E"), None);
    }

    #[test]
    fn superseded_freeze_cannot_store_or_serve_frames() {
        let state = FreezeState::default();
        let old = state.begin();
        let new = state.begin();
        assert!(!state.is_current(old));
        assert!(state.store(old, Vec::new()).is_none());
        assert!(state.store(new, Vec::new()).is_some());
        assert!(state.get(old).is_none());
        assert!(state.get(new).is_some());
        assert!(state.take().is_some());
        assert!(
            state.get(new).is_none(),
            "taken frames are no longer served"
        );
    }
}
