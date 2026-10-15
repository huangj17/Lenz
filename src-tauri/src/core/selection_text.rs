//! Read the current selection before Lenz takes focus. Never accept our own UI as a source.
use crate::core::popup_position::ScreenRect;

#[derive(Clone, Debug)]
pub struct SelectedText {
    pub text: String,
    pub bounds: Option<ScreenRect>,
}

/// Identity of the foreground source, used to reject focus changes during a copy probe.
#[cfg(target_os = "macos")]
pub fn foreground_id() -> Option<u64> {
    let app = objc2_app_kit::NSWorkspace::sharedWorkspace().frontmostApplication()?;
    let pid = app.processIdentifier() as u32;
    (pid != std::process::id()).then_some(pid as u64)
}

#[cfg(target_os = "windows")]
pub fn foreground_id() -> Option<u64> {
    use windows::Win32::UI::WindowsAndMessaging::{GetForegroundWindow, GetWindowThreadProcessId};
    unsafe {
        let window = GetForegroundWindow();
        let mut pid = 0;
        GetWindowThreadProcessId(window, Some(&mut pid));
        (pid != 0 && pid != std::process::id()).then_some(window.0 as usize as u64)
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn foreground_id() -> Option<u64> {
    None
}

/// AX can block on another process; call on a blocking worker.
#[cfg(target_os = "macos")]
pub fn read_selected_text() -> Option<SelectedText> {
    macos::read()
}

#[cfg(target_os = "macos")]
mod macos {
    use super::*;
    use objc2_core_foundation::{CFString, CGRect};
    use std::ffi::c_void;
    use std::ptr;

    #[link(name = "ApplicationServices", kind = "framework")]
    extern "C" {
        fn AXUIElementCreateApplication(pid: i32) -> *const c_void;
        fn AXUIElementCopyAttributeValue(
            element: *const c_void,
            attribute: *const CFString,
            value: *mut *const c_void,
        ) -> i32;
        fn AXUIElementCopyParameterizedAttributeValue(
            element: *const c_void,
            attribute: *const CFString,
            parameter: *const c_void,
            value: *mut *const c_void,
        ) -> i32;
        fn AXUIElementSetMessagingTimeout(element: *const c_void, timeout: f32) -> i32;
        fn AXValueGetTypeID() -> usize;
        fn AXValueGetValue(value: *const c_void, kind: u32, output: *mut c_void) -> bool;
    }
    #[link(name = "CoreFoundation", kind = "framework")]
    extern "C" {
        fn CFRelease(cf: *const c_void);
        fn CFGetTypeID(cf: *const c_void) -> usize;
        fn CFStringGetTypeID() -> usize;
    }

    struct Owned(*const c_void);
    impl Drop for Owned {
        fn drop(&mut self) {
            unsafe {
                CFRelease(self.0);
            }
        }
    }
    unsafe fn attribute(element: &Owned, name: &str) -> Option<Owned> {
        let key = name;
        let name = CFString::from_str(name);
        let mut value = ptr::null();
        let error = AXUIElementCopyAttributeValue(element.0, &*name, &mut value);
        let value = (!value.is_null()).then(|| Owned(value));
        if error == 0 {
            value
        } else {
            log::debug!("AX attribute {key} unavailable: {error}");
            None
        }
    }

    unsafe fn bounds(element: &Owned) -> Option<ScreenRect> {
        let range = attribute(element, "AXSelectedTextRange")?;
        if CFGetTypeID(range.0) != AXValueGetTypeID() {
            return None;
        }
        let name = CFString::from_str("AXBoundsForRange");
        let mut value = ptr::null();
        let error =
            AXUIElementCopyParameterizedAttributeValue(element.0, &*name, range.0, &mut value);
        let owned = (!value.is_null()).then(|| Owned(value))?;
        if error != 0 || CFGetTypeID(owned.0) != AXValueGetTypeID() {
            return None;
        }
        let mut rect = CGRect::default();
        // kAXValueCGRectType = 3. AX geometry has a top-left origin in screen points.
        if !AXValueGetValue(owned.0, 3, (&mut rect as *mut CGRect).cast()) {
            return None;
        }
        ScreenRect::new(
            rect.origin.x,
            rect.origin.y,
            rect.size.width,
            rect.size.height,
        )
    }

    pub fn read() -> Option<SelectedText> {
        let source = foreground_id()?;
        unsafe {
            // Query the actual foreground application, rather than a stale system-wide focus.
            let raw = AXUIElementCreateApplication(source as i32);
            if raw.is_null() {
                return None;
            }
            let app = Owned(raw);
            AXUIElementSetMessagingTimeout(app.0, 0.3);
            let focused = attribute(&app, "AXFocusedUIElement")?;
            AXUIElementSetMessagingTimeout(focused.0, 0.3);
            let value = attribute(&focused, "AXSelectedText")?;
            if CFGetTypeID(value.0) != CFStringGetTypeID() {
                return None;
            }
            let text = (&*(value.0 as *const CFString))
                .to_string()
                .trim()
                .to_string();
            let bounds = bounds(&focused);
            if text.is_empty() || foreground_id() != Some(source) {
                return None;
            }
            Some(SelectedText { text, bounds })
        }
    }
}

/// Windows:UI Automation 直读焦点元素的选区,不支持时回退剪贴板。
#[cfg(target_os = "windows")]
pub fn read_selected_text() -> Option<SelectedText> {
    use windows::Win32::{
        System::Com::{
            CoCreateInstance, CoInitializeEx, CoUninitialize, CLSCTX_INPROC_SERVER,
            COINIT_MULTITHREADED,
        },
        UI::Accessibility::{
            CUIAutomation, IUIAutomation, IUIAutomationTextPattern, UIA_TextPatternId,
        },
    };
    unsafe {
        CoInitializeEx(None, COINIT_MULTITHREADED).ok().ok()?;
        struct Apartment;
        impl Drop for Apartment {
            fn drop(&mut self) {
                unsafe {
                    CoUninitialize();
                }
            }
        }
        let _apartment = Apartment;
        let automation: IUIAutomation =
            CoCreateInstance(&CUIAutomation, None, CLSCTX_INPROC_SERVER).ok()?;
        let element = automation.GetFocusedElement().ok()?;
        if element.CurrentIsPassword().ok()?.as_bool() {
            return None;
        }
        let pattern: IUIAutomationTextPattern =
            element.GetCurrentPatternAs(UIA_TextPatternId).ok()?;
        let selection = pattern.GetSelection().ok()?;
        let mut parts = Vec::new();
        for i in 0..selection.Length().ok()? {
            parts.push(selection.GetElement(i).ok()?.GetText(-1).ok()?.to_string());
        }
        let text = parts.join("\n");
        (!text.trim().is_empty()).then_some(SelectedText { text, bounds: None })
    }
}
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
pub fn read_selected_text() -> Option<SelectedText> {
    None
}

#[cfg(all(test, target_os = "macos"))]
pub(crate) mod tests {
    use super::*;

    #[test]
    #[ignore = "requires the tests/fixtures/selection.txt opened in TextEdit in the foreground and Accessibility permission"]
    fn native_fixture_selection_includes_text_and_screen_bounds() {
        let selected = fixture_selection();
        let bounds = selected
            .bounds
            .expect("text fixture must expose selected range bounds");
        assert!(bounds.w > 0.0 && bounds.h > 0.0);
    }

    pub(crate) fn fixture_selection() -> SelectedText {
        let source = objc2_app_kit::NSWorkspace::sharedWorkspace().frontmostApplication();
        assert!(
            source
                .and_then(|app| app.bundleIdentifier())
                .is_some_and(|id| id.to_string() == "com.apple.TextEdit"),
            "activate TextEdit with tests/fixtures/selection.txt selected before running this test"
        );
        let selected = read_selected_text().expect("fixture selection must be readable");
        assert!(
            selected.text == include_str!("../../../tests/fixtures/selection.txt"),
            "select only the controlled fixture text"
        );
        selected
    }
}
