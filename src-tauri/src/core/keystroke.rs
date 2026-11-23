//! 合成系统「复制」按键(spec 05 §3)。
//!
//! 划词翻译快捷键模式下,Alt+D 触发时应自动把前台应用中**当前选中的文本**
//! (双击选中 / 拖拽划词)送入剪贴板,再读出翻译。这里向前台应用合成一次
//! 复制快捷键:macOS = `Cmd+C`,Windows = `Ctrl+C`。
//!
//! - macOS:`CGEvent` 合成键盘事件。**需「辅助功能(Accessibility)」权限**,
//!   未授权时系统会静默丢弃事件(函数仍返回 Ok);调用方据「复制前后剪贴板是否
//!   变化」判断是否真有选中文本,不会回显无关旧结果。
//! - Windows:`SendInput` 合成 `Ctrl+C`。

use crate::error::LenzError;

/// 当前是否有任一「快捷键修饰键」(⇧⌃⌥⌘ 或 Fn)处于物理按下状态。
///
/// 划词快捷键(Alt+D)触发时,用户的 Option 往往仍按着;若此刻合成 Cmd+C,被按住的
/// 修饰键会并入合成事件 → 前台应用收到的是 `⌘⌥C` 而非 `⌘C`,复制不会发生。调用方据此
/// 先等触发键松开再合成(spec 05 §3)。Windows 同理:Alt 未松开时注入的 Ctrl+C 会变成
/// Ctrl+Alt+C,不仅复制失败,在 AltGr 布局下还会输入字符替换选中文本。
pub fn modifiers_held() -> bool {
    #[cfg(target_os = "macos")]
    {
        macos::modifiers_held()
    }
    #[cfg(target_os = "windows")]
    {
        windows_impl::modifiers_held()
    }
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    {
        false
    }
}

/// 向前台应用合成一次系统「复制」快捷键。
///
/// 返回 `Ok` 仅表示事件已派发;Windows 附带输入时间戳。**不保证**前台应用真的复制了内容
/// (无选中 / 权限不足时剪贴板不会变化)。调用方应在其后比较剪贴板内容。
pub fn copy_selection() -> Result<Option<u32>, LenzError> {
    #[cfg(target_os = "macos")]
    {
        macos::copy().map(|()| None)
    }
    #[cfg(target_os = "windows")]
    {
        windows_impl::copy().map(Some)
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

    /// 读取硬件修饰键标志,判断是否有 ⇧⌃⌥⌘/Fn 仍被按下。
    pub fn modifiers_held() -> bool {
        let cur = CGEventSource::flags_state(CGEventSourceStateID::HIDSystemState);
        let mods = CGEventFlags::MaskShift
            | CGEventFlags::MaskControl
            | CGEventFlags::MaskAlternate
            | CGEventFlags::MaskCommand
            | CGEventFlags::MaskSecondaryFn;
        cur.intersects(mods)
    }

    pub fn copy() -> Result<(), LenzError> {
        // 私有事件源不继承物理 Option 等状态,也不污染用于检查用户输入的 HID 计数。
        let source = CGEventSource::new(CGEventSourceStateID::Private)
            .ok_or_else(|| LenzError::internal("create copy event source failed"))?;
        let src = Some(&*source);

        let down = CGEvent::new_keyboard_event(src, KEY_C, true)
            .ok_or_else(|| LenzError::internal("create Cmd+C key-down failed"))?;
        CGEvent::set_flags(Some(&down), CGEventFlags::MaskCommand);
        let up = CGEvent::new_keyboard_event(src, KEY_C, false)
            .ok_or_else(|| LenzError::internal("create Cmd+C key-up failed"))?;
        CGEvent::set_flags(Some(&up), CGEventFlags::MaskCommand);
        CGEvent::post(CGEventTapLocation::SessionEventTap, Some(&down));
        CGEvent::post(CGEventTapLocation::SessionEventTap, Some(&up));
        Ok(())
    }
}

#[cfg(target_os = "windows")]
mod windows_impl {
    use windows::Win32::UI::Input::KeyboardAndMouse::{
        GetAsyncKeyState, SendInput, INPUT, INPUT_0, INPUT_KEYBOARD, KEYBDINPUT, KEYEVENTF_KEYUP,
        VIRTUAL_KEY, VK_C, VK_CONTROL, VK_LWIN, VK_MENU, VK_RWIN, VK_SHIFT,
    };

    use crate::error::LenzError;

    /// 物理修饰键状态(`GetAsyncKeyState` 最高位 = 当前按下)。
    pub fn modifiers_held() -> bool {
        [VK_SHIFT, VK_CONTROL, VK_MENU, VK_LWIN, VK_RWIN]
            .into_iter()
            .any(|vk| unsafe { GetAsyncKeyState(i32::from(vk.0)) } < 0)
    }

    /// `SendInput` 只注入了前 `sent` 个事件时,需要补发的抬起事件。
    ///
    /// 序列为 Ctrl↓ C↓ C↑ Ctrl↑:已按下但尚未抬起的键必须补发抬起,否则 Ctrl 会保持
    /// 按下状态,用户后续的每次按键都会变成 Ctrl 组合键。
    pub(super) fn pending_releases(sent: usize) -> &'static [VIRTUAL_KEY] {
        match sent {
            1 => &[VK_CONTROL],
            2 => &[VK_C, VK_CONTROL],
            3 => &[VK_CONTROL],
            _ => &[],
        }
    }

    pub fn copy() -> Result<u32, LenzError> {
        #[link(name = "kernel32")]
        extern "system" {
            fn GetTickCount() -> u32;
        }
        // Explicit event time lets the clipboard probe distinguish our injected
        // Ctrl+C from intervening physical input through GetLastInputInfo.
        let stamp = unsafe { GetTickCount() }.max(1);
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
                    time: stamp,
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
        let sent = unsafe { SendInput(&inputs, std::mem::size_of::<INPUT>() as i32) } as usize;
        if sent != inputs.len() {
            let releases: Vec<INPUT> = pending_releases(sent)
                .iter()
                .map(|vk| key(*vk, true))
                .collect();
            if !releases.is_empty() {
                unsafe { SendInput(&releases, std::mem::size_of::<INPUT>() as i32) };
            }
            return Err(LenzError::internal("SendInput Ctrl+C partially failed"));
        }
        Ok(stamp)
    }
}

#[cfg(all(test, target_os = "windows"))]
mod tests {
    use super::windows_impl::pending_releases;
    use windows::Win32::UI::Input::KeyboardAndMouse::{VK_C, VK_CONTROL};

    #[test]
    fn partial_send_input_releases_every_pressed_key() {
        assert!(pending_releases(0).is_empty());
        assert_eq!(pending_releases(1), &[VK_CONTROL]);
        assert_eq!(pending_releases(2), &[VK_C, VK_CONTROL]);
        assert_eq!(pending_releases(3), &[VK_CONTROL]);
        assert!(pending_releases(4).is_empty());
    }
}
