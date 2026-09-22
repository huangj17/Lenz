//! 划词监听(spec 05 §2)。
//!
//! `SelectionListener` 作为 Tauri managed state 注入,生命周期由「托盘开关 /
//! `toggle_selection_listener` 命令 / 启动时 `selection.enabled && autoDetect`」驱动
//! (start / stop / restart)。
//!
//! macOS 实现:在独立线程上跑一个 `CGEventTap`(listen-only,会话级),监听鼠标
//! 左键 down/up。当一次 up 满足「拖拽划词(位移超阈值)或双击选中(clickState≥2)」
//! 时,在光标处 emit [`EVENT_SELECTION_DETECTED`],由前端在该处浮出划词指示图标
//! (真正的复制+翻译延迟到用户 hover/点击图标时,见 spec 05 §6)。
//!
//! ⚠️ CGEventTap 需「辅助功能」权限(`AXIsProcessTrusted`)。未授时不启动监听线程,
//! 改为 emit [`EVENT_PERMISSION_REQUIRED`] 引导授权;授权后重新开启监听即可生效。
//! 注:listen-only tap 不修改/吞掉事件,不影响正常鼠标操作。

use std::sync::Mutex;

use tauri::{AppHandle, Manager};

use crate::config::ConfigStore;

/// `selection://detected` 事件名:监听到一次划词/双击选中手势(候选,待探测确认)。
pub const EVENT_SELECTION_DETECTED: &str = "selection://detected";
/// `selection://dismiss` 事件名:用户点击图标外 → 立刻隐藏划词指示图标。
pub const EVENT_SELECTION_DISMISS: &str = "selection://dismiss";
/// `selection://permission-required` 事件名:缺少辅助功能权限。
pub const EVENT_PERMISSION_REQUIRED: &str = "selection://permission-required";

/// 划词监听器(managed state)。内部按平台持有监听线程句柄。
#[derive(Default)]
pub struct SelectionListener {
    inner: Mutex<Inner>,
}

#[derive(Default)]
struct Inner {
    #[cfg(target_os = "macos")]
    handle: Option<macos::TapHandle>,
    /// 非 macOS 占位,标记是否「逻辑上开启」(便于 is_running 一致返回)。
    #[cfg(not(target_os = "macos"))]
    running: bool,
}

impl SelectionListener {
    pub fn new() -> Self {
        Self::default()
    }

    /// 启动监听。已在运行则先停后启(幂等)。无权限时不启动,仅 emit 引导事件。
    pub fn start(&self, app: &AppHandle) {
        let mut inner = self.inner.lock().unwrap();
        #[cfg(target_os = "macos")]
        {
            if inner.handle.is_some() {
                return;
            }
            inner.handle = macos::start(app);
        }
        #[cfg(not(target_os = "macos"))]
        {
            let _ = app;
            inner.running = true;
            log::debug!("selection listener start: 非 macOS 平台暂未实现(M5 Windows)");
        }
    }

    /// 停止监听(若在运行)。
    pub fn stop(&self) {
        let mut inner = self.inner.lock().unwrap();
        #[cfg(target_os = "macos")]
        {
            if let Some(handle) = inner.handle.take() {
                handle.stop();
            }
        }
        #[cfg(not(target_os = "macos"))]
        {
            inner.running = false;
        }
    }

    /// 配置变更后重启(先停后启)。
    pub fn restart(&self, app: &AppHandle) {
        self.stop();
        self.start(app);
    }

    /// 当前是否在监听。
    pub fn is_running(&self) -> bool {
        let inner = self.inner.lock().unwrap();
        #[cfg(target_os = "macos")]
        {
            inner.handle.is_some()
        }
        #[cfg(not(target_os = "macos"))]
        {
            inner.running
        }
    }
}

/// 据当前配置启停划词监听:`selection.enabled`(即托盘「划词监听」总开关,spec 05
/// §1「自动触发受总开关 selection.enabled 控制」)为真则开,否则停。幂等,可在
/// 「启动 / 托盘开关 / 设置变更(config://changed)」处反复调用。
///
/// 默认 `enabled=false`(opt-in):用户勾选托盘「划词监听」后才开启(并按需授权辅助
/// 功能);Alt+D 快捷键划词不受此开关影响,始终可用。
pub fn sync(app: &AppHandle) {
    let want = match app.state::<ConfigStore>().get() {
        Ok(cfg) => cfg.selection.enabled,
        Err(e) => {
            log::warn!("selection sync: 读配置失败 {e}");
            return;
        }
    };
    let listener = app.state::<SelectionListener>();
    if want {
        listener.start(app);
    } else {
        listener.stop();
        crate::core::indicator::hide(app);
    }
}

/// 是否已获划词监听所需的系统权限(macOS「辅助功能」;其它平台恒 true)。
/// 供命令层在「启用监听但未授权」时引导用户去系统设置。
pub fn permission_granted() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::ax_trusted()
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use std::cell::Cell;
    use std::ffi::c_void;
    use std::ptr::NonNull;
    use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
    use std::sync::mpsc::{self, RecvTimeoutError};
    use std::sync::Arc;
    use std::thread::JoinHandle;

    use objc2_core_foundation::{kCFRunLoopCommonModes, CFMachPort, CFRunLoop};
    use objc2_core_graphics::{
        CGEvent, CGEventField, CGEventMask, CGEventTapLocation, CGEventTapOptions,
        CGEventTapPlacement, CGEventTapProxy, CGEventType,
    };
    use tauri::{AppHandle, Emitter};

    use crate::types::{PermissionRequired, SelectionDetected};

    use super::{EVENT_PERMISSION_REQUIRED, EVENT_SELECTION_DETECTED, EVENT_SELECTION_DISMISS};

    /// 判定「拖拽划词」的最小位移(point);小于此视为普通点击。
    const DRAG_THRESHOLD: f64 = 4.0;

    /// 监听线程共享状态:发布当前线程 run loop 指针供跨线程 stop。
    struct Shared {
        /// `*const CFRunLoop` 以 usize 形式发布;0 表示尚未进入 run loop。
        runloop_ptr: AtomicUsize,
        /// stop 可能早于 run loop 发布;线程发布后必须再次检查此标志。
        stop_requested: AtomicBool,
    }

    /// 监听线程句柄。`stop` 会打断 run loop 并 join 线程。
    pub struct TapHandle {
        shared: Arc<Shared>,
        join: Option<JoinHandle<()>>,
    }

    impl TapHandle {
        pub fn stop(mut self) {
            self.shared.stop_requested.store(true, Ordering::SeqCst);
            // 打断监听线程的 CFRunLoopRun(CFRunLoopStop 可跨线程安全调用)。
            let ptr = self.shared.runloop_ptr.load(Ordering::SeqCst);
            if ptr != 0 {
                let rl = unsafe { &*(ptr as *const CFRunLoop) };
                rl.stop();
            }
            if let Some(join) = self.join.take() {
                let _ = join.join();
            }
        }
    }

    /// 传给 CGEventTap 回调的上下文(仅在监听线程内访问,故用 Cell 即可)。
    struct TapCtx {
        app: AppHandle,
        /// 最近一次 mouseDown 的位置(用于判断是否拖拽)。
        down: Cell<Option<(f64, f64)>>,
        /// tap 的 `CFMachPort` 指针(usize),用于在被系统禁用后重新启用。
        tap_ptr: Cell<usize>,
    }

    /// 启动监听线程;无权限时返回 None 并 emit 引导事件。
    pub fn start(app: &AppHandle) -> Option<TapHandle> {
        if !ax_trusted() {
            log::warn!("划词监听:缺少辅助功能权限,emit permission-required");
            let _ = app.emit(
                EVENT_PERMISSION_REQUIRED,
                PermissionRequired {
                    platform: "macos".into(),
                    detail: "需要『辅助功能』权限才能监听划词选中".into(),
                },
            );
            return None;
        }

        let shared = Arc::new(Shared {
            runloop_ptr: AtomicUsize::new(0),
            stop_requested: AtomicBool::new(false),
        });
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let app = app.clone();
        let shared_thread = shared.clone();
        let join = std::thread::Builder::new()
            .name("lenz-selection-tap".into())
            .spawn(move || run_tap_loop(app, shared_thread, ready_tx))
            .ok()?;
        let handle = TapHandle {
            shared,
            join: Some(join),
        };
        match ready_rx.recv_timeout(std::time::Duration::from_secs(2)) {
            Ok(()) => Some(handle),
            Err(RecvTimeoutError::Disconnected) => {
                // 初始化失败,线程已返回;join 只负责回收句柄。
                if let Some(join) = handle.join {
                    let _ = join.join();
                }
                None
            }
            Err(RecvTimeoutError::Timeout) => {
                // 不在调用线程无限等待卡住的系统 API。置取消标志后分离线程;若它稍后
                // 完成初始化,会观察标志并直接退出,不会进入 CFRunLoopRun。
                handle.shared.stop_requested.store(true, Ordering::SeqCst);
                log::error!("划词监听初始化超时,已请求取消");
                None
            }
        }
    }

    /// 监听线程主体:建 tap → 挂到本线程 run loop → run(阻塞至 stop)。
    fn run_tap_loop(app: AppHandle, shared: Arc<Shared>, ready: mpsc::SyncSender<()>) {
        let ctx = Box::new(TapCtx {
            app,
            down: Cell::new(None),
            tap_ptr: Cell::new(0),
        });
        let ctx_ptr = &*ctx as *const TapCtx as *mut c_void;

        // 仅关心左键 down/up。
        let mask: CGEventMask =
            (1u64 << CGEventType::LeftMouseDown.0) | (1u64 << CGEventType::LeftMouseUp.0);

        // 会话级、listen-only:不改写/不吞事件,只旁观。
        let tap = unsafe {
            CGEvent::tap_create(
                CGEventTapLocation::SessionEventTap,
                CGEventTapPlacement::HeadInsertEventTap,
                CGEventTapOptions::ListenOnly,
                mask,
                Some(tap_callback),
                ctx_ptr,
            )
        };
        let Some(tap) = tap else {
            log::error!("CGEventTapCreate 失败(辅助功能权限?)");
            return;
        };
        ctx.tap_ptr.set(&*tap as *const CFMachPort as usize);

        let Some(source) = CFMachPort::new_run_loop_source(None, Some(&tap), 0) else {
            log::error!("CFMachPortCreateRunLoopSource 失败");
            return;
        };
        let Some(runloop) = CFRunLoop::current() else {
            log::error!("CFRunLoopGetCurrent 失败");
            return;
        };
        runloop.add_source(Some(&source), unsafe { kCFRunLoopCommonModes });
        CGEvent::tap_enable(&tap, true);

        // 发布 run loop 指针,允许 stop() 打断;然后阻塞运行。
        shared
            .runloop_ptr
            .store(&*runloop as *const CFRunLoop as usize, Ordering::SeqCst);
        let _ = ready.send(());
        log::info!("划词监听已启动(CGEventTap)");
        if !shared.stop_requested.load(Ordering::SeqCst) {
            CFRunLoop::run();
        }

        // run() 返回(被 stop 打断):清理。tap/source/runloop/ctx 在此作用域结束时释放。
        shared.runloop_ptr.store(0, Ordering::SeqCst);
        drop(ctx);
        log::info!("划词监听已停止");
    }

    /// CGEventTap 回调(运行在监听线程)。listen-only,返回值被忽略,原样回传事件。
    unsafe extern "C-unwind" fn tap_callback(
        _proxy: CGEventTapProxy,
        etype: CGEventType,
        event: NonNull<CGEvent>,
        user_info: *mut c_void,
    ) -> *mut CGEvent {
        let ctx = unsafe { &*(user_info as *const TapCtx) };
        let ev = unsafe { event.as_ref() };

        if etype == CGEventType::LeftMouseDown {
            let p = CGEvent::location(Some(ev));
            ctx.down.set(Some((p.x, p.y)));
            // 图标可见且点击落在图标外 → 立刻隐藏(失去选区即消失,spec 05 §6)。
            // 落在图标内则放行,交由 WebView 的点击触发翻译。
            if !crate::core::indicator::hit_test(p.x, p.y) {
                crate::core::indicator::cancel_pending();
                let _ = ctx.app.emit(EVENT_SELECTION_DISMISS, ());
            }
        } else if etype == CGEventType::LeftMouseUp {
            let p = CGEvent::location(Some(ev));
            let click = CGEvent::integer_value_field(Some(ev), CGEventField::MouseEventClickState);
            let dragged = ctx
                .down
                .get()
                .map(|(dx, dy)| ((p.x - dx).powi(2) + (p.y - dy).powi(2)).sqrt() >= DRAG_THRESHOLD)
                .unwrap_or(false);
            ctx.down.set(None);
            // 拖拽划词 或 双击及以上选中 → 认为产生了文本选区。
            if dragged || click >= 2 {
                let _ = ctx.app.emit(
                    EVENT_SELECTION_DETECTED,
                    SelectionDetected { x: p.x, y: p.y },
                );
            }
        } else if etype == CGEventType::TapDisabledByTimeout
            || etype == CGEventType::TapDisabledByUserInput
        {
            // 系统偶尔会临时禁用 tap(超时/高负载),需重新启用。
            let ptr = ctx.tap_ptr.get();
            if ptr != 0 {
                let tap = unsafe { &*(ptr as *const CFMachPort) };
                CGEvent::tap_enable(tap, true);
            }
        }

        event.as_ptr()
    }

    /// 是否已获「辅助功能」权限(`AXIsProcessTrusted`,ApplicationServices 框架)。
    pub fn ax_trusted() -> bool {
        #[link(name = "ApplicationServices", kind = "framework")]
        extern "C" {
            fn AXIsProcessTrusted() -> u8;
        }
        unsafe { AXIsProcessTrusted() != 0 }
    }
}
