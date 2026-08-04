//! 直读「当前选中文本」(spec 05 §3/§6,2026-06-18 方案重做)。
//!
//! 划词 / 双击选中 + Alt+D(或点划词图标)时,优先用 macOS **Accessibility API**
//! 直接读取系统级焦点元素的 `AXSelectedText`,而非合成 `Cmd+C` + 比对剪贴板。后者有
//! 多个固有失败模式:① 触发键(Alt+D 的 Option)仍按住会把合成的 Cmd+C 污染成 ⌘⌥C;
//! ② 重复选中同一段文字时剪贴板内容不变,被误判为「无新选区」;③ 依赖焦点回切时序;
//! ④ 会改写用户剪贴板。AX 直读把这些一并规避。
//!
//! AX 取不到(部分 Chrome/Electron 不暴露 `AXSelectedText`)时返回 `None`,调用方
//! 回退到剪贴板合成方案兜底。
//!
//! ⚠️ 需「辅助功能」权限(与划词监听同权限);未授时 AX 调用返回错误码 → `None`。

/// 读取前台应用中**当前选中文本**;无选中 / 不支持 / 无权限时返回 `None`。
///
/// 可能对无响应的目标应用产生同步 IPC 阻塞,调用方应放到 `spawn_blocking` 等非 async
/// 工作线程上执行(见 `core/screenshot.rs`)。
#[cfg(target_os = "macos")]
pub fn read_selected_text() -> Option<String> {
    use std::ffi::c_void;
    use std::ptr::{self, NonNull};

    use objc2_core_foundation::{CFRetained, CFString};

    // AXUIElementRef = CFTypeRef(不透明);属性名传 CFStringRef。
    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateSystemWide() -> *const c_void;
        fn AXUIElementCopyAttributeValue(
            element: *const c_void,
            attribute: *const CFString,
            value: *mut *const c_void,
        ) -> i32; // AXError,0 = kAXErrorSuccess
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: *const c_void);
        fn CFGetTypeID(cf: *const c_void) -> usize; // CFTypeID
        fn CFStringGetTypeID() -> usize;
    }

    // kAX*Attribute 常量本质就是这两个字符串,直接构造避免依赖 extern 静态符号。
    let focused_attr = CFString::from_str("AXFocusedUIElement");
    let selected_attr = CFString::from_str("AXSelectedText");

    unsafe {
        let system_wide = AXUIElementCreateSystemWide();
        if system_wide.is_null() {
            return None;
        }

        // systemWide → 焦点元素。
        let mut focused: *const c_void = ptr::null();
        let err = AXUIElementCopyAttributeValue(
            system_wide,
            &*focused_attr as *const CFString,
            &mut focused,
        );
        CFRelease(system_wide);
        if err != 0 || focused.is_null() {
            return None;
        }

        // 焦点元素 → 选中文本(CFStringRef,+1 引用)。
        let mut value: *const c_void = ptr::null();
        let err =
            AXUIElementCopyAttributeValue(focused, &*selected_attr as *const CFString, &mut value);
        CFRelease(focused);
        if err != 0 || value.is_null() {
            return None;
        }

        // 防御:确认返回的确是 CFString,否则按 CFType 释放后放弃(避免按字符串误读)。
        if CFGetTypeID(value) != CFStringGetTypeID() {
            CFRelease(value);
            return None;
        }

        // 接管 +1 引用(CFRetained 析构时 CFRelease),转 Rust String。
        let cf = CFRetained::from_raw(NonNull::new_unchecked(value as *mut CFString));
        let text = cf.to_string();
        let trimmed = text.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    }
}

/// 非 macOS:暂不支持 AX 直读,返回 `None`(调用方回退剪贴板方案)。
// TODO(M5 Windows): UI Automation `TextPattern.GetSelection` 或回退剪贴板。
#[cfg(not(target_os = "macos"))]
pub fn read_selected_text() -> Option<String> {
    None
}
