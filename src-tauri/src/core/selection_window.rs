//! 选区蒙层窗口的运行时创建 / 销毁(spec 02 §3 / spec 09 §4)。
//!
//! **每块显示器**各创建一个透明全屏窗口 `selection-{i}`,精确铺满该屏;前端在其上
//! 用 canvas 绘制遮罩 + 框选(见 `src/windows/selection`)。框选完成 / 取消由前端
//! emit `capture://region-selected` / `capture://region-cancelled`,Rust 侧据此销毁
//! 全部蒙层窗口并接续 F2/F3 流程。
//!
//! ⚠️ 为何「每屏一窗」而非单个跨虚拟屏的窗口:macOS「调度中心 → 显示器各自拥有
//! 独立 Space」(系统默认开启)会把横跨多屏的窗口强制夹到单块屏幕上,导致蒙层只
//! 出现在某一屏、其余屏框选不到。按屏拆窗可规避该限制,在 Windows 上也同样成立。
//!
//! 选区开始前先定格整屏(见 `core::freeze`):有定格图的窗口以其为背景,前端画好后经
//! `selection_ready` 再显示;定格失败或定格后新接入的屏退回透明实时蒙层。
//!
//! macOS 要求窗口创建在主线程,故 `open` 把构建动作 dispatch 到主线程执行。

use std::time::Duration;

use tauri::{AppHandle, Manager, Monitor, WebviewUrl, WebviewWindow, WebviewWindowBuilder};

use crate::core::capture::{self, LogicalBounds};
use crate::core::freeze::{self, FreezeState, FrozenScreen};
use crate::error::LenzError;
use crate::types::TriggerKind;

/// selection 窗口 label 前缀;每块显示器一个窗口,label 为 `selection-{i}`。
pub const SELECTION_LABEL_PREFIX: &str = "selection";

/// 托盘菜单入口的定格延迟:等托盘菜单收起,避免把菜单本身截进定格图。
pub const TRAY_MENU_CLEAR_DELAY: Duration = Duration::from_millis(300);

/// 定格前隐藏 Lenz 自己的浮层后,等待其从屏幕消失(主线程派发 + 合成器出帧)。
const OWN_WINDOWS_CLEAR_DELAY: Duration = Duration::from_millis(150);

/// 前端迟迟未报告定格背景就绪(如图片解码失败)时,超时后仍显示窗口,保证可取消。
const FRAME_READY_TIMEOUT: Duration = Duration::from_millis(1500);

/// 承载键盘(ESC)的窗口。
fn focus_label() -> String {
    format!("{SELECTION_LABEL_PREFIX}-0")
}

/// 定格整屏后为每块显示器打开选区蒙层窗口;已存在则先销毁再重建(spec 02 §4)。
pub fn open(app: &AppHandle, trigger: TriggerKind) -> Result<(), LenzError> {
    open_after(app, trigger, Duration::ZERO)
}

/// 同 [`open`],但先等待 `delay` 再定格(托盘入口等菜单收起)。
///
/// 定格与建窗都在后台进行,故本函数立即返回,真正失败仅记日志。
pub fn open_after(app: &AppHandle, trigger: TriggerKind, delay: Duration) -> Result<(), LenzError> {
    let own_visible = own_windows_visible(app);
    hide_all(app);
    crate::core::screenshot::hide_for_capture(app);
    let token = app.state::<FreezeState>().begin();
    let app = app.clone();
    tauri::async_runtime::spawn(async move {
        let wait = if own_visible {
            delay.max(OWN_WINDOWS_CLEAR_DELAY)
        } else {
            delay
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
        if !app.state::<FreezeState>().is_current(token) {
            return;
        }
        // 定格失败(如缺屏幕录制权限)时退回实时蒙层,沿用框选后截屏的错误提示路径。
        let screen = match capture::freeze_all_with_timeout().await {
            Ok(frames) => app.state::<FreezeState>().store(token, frames),
            Err(_) => None,
        };
        if !app.state::<FreezeState>().is_current(token) {
            return;
        }
        let handle = app.clone();
        let dispatched = app.run_on_main_thread(move || {
            if !handle.state::<FreezeState>().is_current(token) {
                return;
            }
            if let Err(e) = open_inner(&handle, trigger, token, screen.as_deref()) {
                log::error!("open selection window failed: {e}");
            }
        });
        if let Err(e) = dispatched {
            log::error!("dispatch open selection failed: {e}");
        }
    });
    Ok(())
}

/// Lenz 自己的浮层(popup / 划词图标 / 覆盖层 / 旧选区)是否在屏幕上。
fn own_windows_visible(app: &AppHandle) -> bool {
    let prefix = format!("{SELECTION_LABEL_PREFIX}-");
    app.webview_windows().iter().any(|(label, window)| {
        (matches!(label.as_str(), "popup" | "indicator" | "overlay") || label.starts_with(&prefix))
            && window.is_visible().unwrap_or(false)
    })
}

/// 主线程内实际构建窗口:遍历显示器,每块各建一个铺满该屏的蒙层窗口。
fn open_inner(
    app: &AppHandle,
    trigger: TriggerKind,
    token: u64,
    screen: Option<&FrozenScreen>,
) -> Result<(), LenzError> {
    // 旧窗口残留则同步销毁,避免 label 冲突。
    close(app)?;

    let trig = match trigger {
        TriggerKind::Overlay => "overlay",
        _ => "screenshot",
    };
    let base_url = format!("index.html?window=selection&trigger={trig}");

    // 每块显示器一窗:用物理坐标精确铺满该屏(规避 macOS 独立 Space 对跨屏窗口的夹取)。
    let monitors = query_monitors(app)?;
    let mut awaiting_frames = false;
    for (i, m) in monitors.iter().enumerate() {
        let label = format!("{SELECTION_LABEL_PREFIX}-{i}");
        let frame = screen.and_then(|screen| {
            capture::frame_for_monitor(&screen.frames, monitor_logical_bounds(m))
                .map(|index| freeze::frame_key(screen.token, index))
        });
        let url = match &frame {
            Some(key) => format!("{base_url}&frame={key}"),
            None => base_url.clone(),
        };
        let win = WebviewWindowBuilder::new(app, label.as_str(), WebviewUrl::App(url.into()))
            .transparent(true)
            .decorations(false)
            .always_on_top(true)
            .skip_taskbar(true)
            .resizable(false)
            // 仅首个窗口取焦(承载键盘 ESC)。
            .focused(i == 0)
            // 关键:接受「首次鼠标」。多屏时只有 i==0 是 key 窗口,鼠标移到别的屏的
            // 蒙层窗口(非活动)上,macOS 默认会把第一下 mousedown 吞去激活该窗口,
            // 导致「切屏后要先点一下才能框选」。acceptsFirstMouse=YES 让非活动窗口的
            // 首击直接派发给 WebView 内容,切屏即可框选(spec 02 §3;仅 macOS 生效)。
            .accept_first_mouse(true)
            // 先隐藏构建,定位/定尺后再 show,避免默认位置闪现。
            .visible(false)
            .build()
            .map_err(|e| LenzError::internal(format!("build {label}: {e}")))?;

        #[cfg(target_os = "macos")]
        raise_above_system_ui(&win);
        position_and_size(&win, m)
            .map_err(|e| LenzError::internal(format!("place {label}: {e}")))?;
        if frame.is_some() {
            // 等前端画好定格背景再显示,避免先露出一帧透明蒙层下的实时画面。
            awaiting_frames = true;
        } else {
            reveal(&win);
        }
    }
    if awaiting_frames {
        let app = app.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(FRAME_READY_TIMEOUT).await;
            if !app.state::<FreezeState>().is_current(token) {
                return;
            }
            let prefix = format!("{SELECTION_LABEL_PREFIX}-");
            for (label, window) in app.webview_windows() {
                if label.starts_with(&prefix) && !window.is_visible().unwrap_or(true) {
                    log::warn!("{label} did not report its frozen frame in time; showing anyway");
                    reveal(&window);
                }
            }
        });
    }
    Ok(())
}

/// 显示选区窗口;承载键盘的首屏窗口同时取焦。重复调用无副作用。
pub fn reveal(window: &WebviewWindow) {
    let label = window.label();
    if !label.starts_with(&format!("{SELECTION_LABEL_PREFIX}-"))
        || window.is_visible().unwrap_or(false)
    {
        return;
    }
    if let Err(e) = window.show() {
        log::warn!("show {label} failed: {e}");
        return;
    }
    if label == focus_label() {
        let _ = window.set_focus();
    }
}

/// 把蒙层提到屏保层级,盖住菜单栏、Dock 和仍展开的菜单,定格图之上不再叠实时画面。
#[cfg(target_os = "macos")]
fn raise_above_system_ui(win: &WebviewWindow) {
    const NS_SCREEN_SAVER_WINDOW_LEVEL: isize = 1000;
    let Ok(ptr) = win.ns_window() else {
        return;
    };
    if ptr.is_null() {
        return;
    }
    let ns_window = ptr as *mut objc2::runtime::AnyObject;
    unsafe {
        let _: () = objc2::msg_send![&*ns_window, setLevel: NS_SCREEN_SAVER_WINDOW_LEVEL];
    }
}

/// 显示器的虚拟屏逻辑边界(各屏按自身 `scale_factor` 换算)。
fn monitor_logical_bounds(m: &Monitor) -> LogicalBounds {
    let sf = m.scale_factor();
    let p = m.position();
    let s = m.size();
    (
        p.x as f64 / sf,
        p.y as f64 / sf,
        s.width as f64 / sf,
        s.height as f64 / sf,
    )
}

/// 把蒙层窗口精确铺满目标显示器 `m`。
///
/// ⚠️ macOS 特有坑:NSWindow 用「全局逻辑点」坐标系。窗口是在**当前屏**(触发截图时
/// 光标所在屏,常为小 Retina 内屏)上以隐藏态构建的,若此时直接对它 `set_position`/
/// `set_size` 传**物理**坐标,tao 会按窗口**当前所在屏**的 scale 把物理换算成逻辑
/// (即一律除以内屏的 2.0)。而外接普通 DPI 屏 scale=1.0(逻辑=物理),其物理尺寸被
/// 错误地按 2.0 缩小,加之窗口 scale 变更是异步的,蒙层遂只铺满一小块 —— 混合缩放
/// (小屏 Retina + 外接 27")下尤其明显。故 macOS 改用各屏**自身** scale 换算出的逻辑
/// 坐标(与 `logical_monitor_bounds` 同源),让 NSWindow 拿到正确的全局逻辑点。
/// 且须先定尺再定位:AppKit 改尺寸时固定左下角,先定位会让顶边随高度上移,而蒙层位于
/// 屏保层级,系统不再把它夹回菜单栏下方。
///
/// Windows:Win32 以物理像素做全局定位,直接用物理坐标即精确,无需换算。
fn position_and_size(win: &WebviewWindow, m: &Monitor) -> tauri::Result<()> {
    #[cfg(target_os = "macos")]
    {
        let sf = m.scale_factor();
        let p = m.position();
        let s = m.size();
        win.set_size(tauri::LogicalSize::new(
            s.width as f64 / sf,
            s.height as f64 / sf,
        ))?;
        win.set_position(tauri::LogicalPosition::new(
            p.x as f64 / sf,
            p.y as f64 / sf,
        ))?;
    }
    #[cfg(not(target_os = "macos"))]
    {
        win.set_position(*m.position())?;
        win.set_size(*m.size())?;
    }
    Ok(())
}

/// 销毁全部选区蒙层窗口(若存在)。`destroy` 同步移除,可跨线程调用。
pub fn close(app: &AppHandle) -> Result<(), LenzError> {
    let prefix = format!("{SELECTION_LABEL_PREFIX}-");
    for (label, w) in app.webview_windows() {
        if label == SELECTION_LABEL_PREFIX || label.starts_with(&prefix) {
            w.destroy()
                .map_err(|e| LenzError::internal(format!("destroy {label}: {e}")))?;
        }
    }
    Ok(())
}

/// 取消选区:销毁蒙层并丢弃本次定格图。
pub fn cancel(app: &AppHandle) -> Result<(), LenzError> {
    app.state::<FreezeState>().clear();
    close(app)
}

/// 立即隐藏全部选区蒙层窗口(orderOut)。截图前先隐藏:`hide` / `destroy` 都是投递到
/// 主线程的异步操作,隐藏能更快地让蒙层无残帧地从屏幕消失,避免被截入或闪现白底
/// (spec 02 §7)。
pub fn hide_all(app: &AppHandle) {
    let prefix = format!("{SELECTION_LABEL_PREFIX}-");
    for (label, w) in app.webview_windows() {
        if label == SELECTION_LABEL_PREFIX || label.starts_with(&prefix) {
            let _ = w.hide();
        }
    }
}

/// 查询全部显示器(走任一已存在窗口,避免依赖 AppHandle 上不稳定的 monitor API)。
fn query_monitors(app: &AppHandle) -> Result<Vec<Monitor>, LenzError> {
    let base = app
        .get_webview_window("popup")
        .or_else(|| app.get_webview_window("main"))
        .ok_or_else(|| LenzError::internal("no base window for monitor query"))?;
    let monitors = base
        .available_monitors()
        .map_err(|e| LenzError::internal(format!("available_monitors: {e}")))?;
    if monitors.is_empty() {
        return Err(LenzError::internal("no monitor found"));
    }
    Ok(monitors)
}

/// 列出各显示器的逻辑边界 `(x, y, w, h)`。
///
/// 各屏以自身 `scale_factor` 把物理坐标换算为逻辑坐标——单屏 / 同 DPR 多屏精确;
/// 混合 DPR 为近似(spec 02 §9 开放问题)。供 popup 定位与夹紧复用(见 screenshot.rs)。
pub fn logical_monitor_bounds(app: &AppHandle) -> Result<Vec<LogicalBounds>, LenzError> {
    Ok(query_monitors(app)?
        .iter()
        .map(monitor_logical_bounds)
        .collect())
}
