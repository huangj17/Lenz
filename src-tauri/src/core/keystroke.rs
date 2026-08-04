//! 合成系统「复制」按键(spec 05 §3)。
//!
//! 划词翻译快捷键模式下,Alt+D 触发时应自动把前台应用中**当前选中的文本**
//! (双击选中 / 拖拽划词)送入剪贴板,再读出翻译。这里向前台应用合成一次
//! 复制快捷键:macOS = `Cmd+C`,Windows = `Ctrl+C`。
//!
//! - macOS:`CGEvent` 合成键盘事件。**需「辅助功能(Accessibility)」权限**,
//!   未授权时系统会静默丢弃事件(函数仍返回 Ok);调用方据「复制前后剪贴板是否
//!   变化」判断是否真有选中文本,故权限缺失会自然退化为「读当前剪贴板」。
//! - Windows:`SendInput` 合成 `Ctrl+C`。

use crate::error::LenzError;

/// 当前是否有任一「快捷键修饰键」(⇧⌃⌥⌘ 或 Fn)处于物理按下状态。
///
/// 划词快捷键(Alt+D)触发时,用户的 Option 往往仍按着;若此刻合成 Cmd+C,被按住的
/// 修饰键会并入合成事件 → 前台应用收到的是 `⌘⌥C` 而非 `⌘C`,复制不会发生。调用方据此
/// 先等触发键松开再合成(spec 05 §3)。非 macOS 恒返回 false。
pub fn modifiers_held() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::modifiers_held()
    }
    #[cfg(not(target_os = "macos"))]
    {
        false
    }
}

/// 向前台应用合成一次系统「复制」快捷键。
///
/// 返回 `Ok` 仅表示事件已派发,**不保证**前台应用真的复制了内容
/// (无选中 / 权限不足时剪贴板不会变化)。调用方应在其后比较剪贴板内容。
pub fn copy_selection() -> Result<(), LenzError> {
    #[cfg(target_os = "macos")]
    {
        macos::copy()
    }
    #[cfg(target_os = "windows")]
    {
        windows_impl::copy()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        Err(LenzError::internal("copy_selection: unsupported platform"))
    }
}

#[cfg(target_os = "macos")]
mod macos {
    use objc2_core_graphics::{
        CGEvent, CGEventFlags, CGEventSource, CGEventSourceStateID, CGEventTapLocation,
    };

    use crate::error::LenzError;

    /// `kVK_ANSI_C` —— 字母 C 的虚拟键码。
    const KEY_C: u16 = 8;

    /// 读取「会话合并态」修饰键标志,判断是否有 ⇧⌃⌥⌘/Fn 仍被按下。
    pub fn modifiers_held() -> bool {
        let cur = CGEventSource::flags_state(CGEventSourceStateID::CombinedSessionState);
        let mods = CGEventFlags::MaskShift
            | CGEventFlags::MaskControl
            | CGEventFlags::MaskAlternate
            | CGEventFlags::MaskCommand
            | CGEventFlags::MaskSecondaryFn;
        cur.intersects(mods)
    }

    pub fn copy() -> Result<(), LenzError> {
        // HID 系统态事件源;构造失败也允许以 None 源发送。
        let source = CGEventSource::new(CGEventSourceStateID::HIDSystemState);
        let src = source.as_deref();

        let down = CGEvent::new_keyboard_event(src, KEY_C, true)
            .ok_or_else(|| LenzError::internal("create Cmd+C key-down failed"))?;
        CGEvent::set_flags(Some(&down), CGEventFlags::MaskCommand);
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&down));

        let up = CGEvent::new_keyboard_event(src, KEY_C, false)
            .ok_or_else(|| LenzError::internal("create Cmd+C key-up failed"))?;
        CGEvent::set_flags(Some(&up), CGEventFlags::MaskCommand);
        CGEvent::post(CGEventTapLocation::HIDEventTap, Some(&up));
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP, VK_C, VK_CONTROL,
    };

    use crate::error::LenzError;

    pub fn copy() -> Result<(), LenzError> {
        let key = |vk, up: bool| INPUT {
            r#type: INPUT_KEYBOARD,
            Anonymous: INPUT_0 {
                ki: KEYBDINPUT {
                    wVk: vk,
                    wScan: 0,
                    dwFlags: if up {
                        KEYEVENTF_KEYUP
                    } else {
                        Default::default()
                    },
                    time: 0,
                    dwExtraInfo: 0,
                },
            },
        };
        // Ctrl↓ C↓ C↑ Ctrl↑
        let inputs = [
            key(VK_CONTROL, false),
            key(VK_C, false),
            key(VK_C, true),
            key(VK_CONTROL, true),
        ];
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) };
        if sent as usize != inputs.len() {
            return Err(LenzError::internal("SendInput Ctrl+C partially failed"));
        }
        Ok(())
    }
}
