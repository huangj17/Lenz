//! Clipboard fallback is a single, bounded transaction. Never write a sentinel.
//! Snapshot every native representation before synthesizing Copy; only restore a
//! stable copy made during this probe. A concurrent writer or user input wins.
use std::sync::Mutex;
use std::time::{Duration, Instant};

static PROBE: Mutex<()> = Mutex::new(());

pub fn read() -> Option<String> {
    // Drop overlapping shortcuts instead of queueing copies into a different app.
    let _guard = PROBE.try_lock().ok()?;
    platform::read()
}

#[cfg(target_os = "macos")]
mod platform {
    use super::*;
    use objc2::runtime::ProtocolObject;
    use objc2_app_kit::{
        NSPasteboard, NSPasteboardItem, NSPasteboardTypeString, NSPasteboardWriting,
    };
    use objc2_core_graphics::{CGEventSource, CGEventSourceStateID, CGEventType};
    use objc2_foundation::NSArray;

    fn inputs() -> [u32; 3] {
        [
            CGEventType::KeyDown,
            CGEventType::LeftMouseDown,
            CGEventType::RightMouseDown,
        ]
        .map(|kind| {
            CGEventSource::counter_for_event_type(CGEventSourceStateID::CombinedSessionState, kind)
        })
    }

    pub fn read() -> Option<String> {
        objc2::rc::autoreleasepool(|_| {
            let board = NSPasteboard::generalPasteboard();
            let before = board.changeCount();
            let mut snapshot = Vec::new();
            let mut size = 0;
            if let Some(items) = board.pasteboardItems() {
                for item in items.iter() {
                    let copy = NSPasteboardItem::new();
                    for kind in item.types().iter() {
                        // Materialize lazy data while its owner still owns the pasteboard.
                        let data = item.dataForType(&kind)?;
                        size += data.length();
                        if size > 16 * 1024 * 1024 || !copy.setData_forType(&data, &kind) {
                            return None;
                        }
                    }
                    snapshot.push(ProtocolObject::<dyn NSPasteboardWriting>::from_retained(
                        copy,
                    ));
                }
            }
            if board.changeCount() != before {
                return None;
            }
            let input_before = inputs();
            crate::core::keystroke::copy_selection().ok()?;
            let deadline = Instant::now() + Duration::from_millis(250);
            while board.changeCount() == before {
                if Instant::now() >= deadline {
                    return None;
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            let copied_version = board.changeCount();
            let text = board
                .stringForType(unsafe { NSPasteboardTypeString })
                .map(|s| s.to_string());
            // Allow delayed formats / another copy to settle before deciding ownership.
            std::thread::sleep(Duration::from_millis(30));
            let input_after = inputs();
            let only_our_key = input_after[0].wrapping_sub(input_before[0]) == 1
                && input_after[1..] == input_before[1..];
            if !super::may_restore(
                before as u64,
                copied_version as u64,
                board.changeCount() as u64,
                only_our_key,
            ) {
                return None;
            }
            board.clearContents();
            if !snapshot.is_empty() && !board.writeObjects(&NSArray::from_retained_slice(&snapshot))
            {
                log::warn!("could not restore native clipboard snapshot");
            }
            text.filter(|s| !s.trim().is_empty())
        })
    }
}

// Windows formats include handles with special ownership (bitmaps, metafiles,
// delayed-rendered objects). Only probe if every format is ordinary global memory;
// otherwise leave the clipboard untouched rather than losing an unsupported item.
#[cfg(target_os = "windows")]
mod platform {
    use super::*;
    use std::ffi::c_void;
    #[link(name = "user32")]
    extern "system" {
        fn CreateWindowExW(
            ex: u32,
            class: *const u16,
            title: *const u16,
            style: u32,
            x: i32,
            y: i32,
            w: i32,
            h: i32,
            parent: *mut c_void,
            menu: *mut c_void,
            instance: *mut c_void,
            param: *mut c_void,
        ) -> *mut c_void;
        fn DestroyWindow(window: *mut c_void) -> i32;
        fn OpenClipboard(owner: *mut c_void) -> i32;
        fn CloseClipboard() -> i32;
        fn EnumClipboardFormats(format: u32) -> u32;
        fn GetClipboardData(format: u32) -> *mut c_void;
        fn SetClipboardData(format: u32, data: *mut c_void) -> *mut c_void;
        fn EmptyClipboard() -> i32;
        fn GetClipboardSequenceNumber() -> u32;
        fn GetLastInputInfo(info: *mut LastInput) -> i32;
    }
    #[link(name = "kernel32")]
    extern "system" {
        fn SetLastError(code: u32);
        fn GetLastError() -> u32;
        fn GlobalSize(memory: *mut c_void) -> usize;
        fn GlobalLock(memory: *mut c_void) -> *mut u8;
        fn GlobalUnlock(memory: *mut c_void) -> i32;
        fn GlobalAlloc(flags: u32, size: usize) -> *mut c_void;
        fn GlobalFree(memory: *mut c_void) -> *mut c_void;
    }
    #[repr(C)]
    struct LastInput {
        size: u32,
        time: u32,
    }
    struct Window(*mut c_void);
    impl Window {
        fn new() -> Option<Self> {
            let class = [83u16, 84, 65, 84, 73, 67, 0]; // STATIC system class
            let hwnd = unsafe {
                CreateWindowExW(
                    0,
                    class.as_ptr(),
                    std::ptr::null(),
                    0,
                    0,
                    0,
                    0,
                    0,
                    -3isize as *mut c_void,
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                    std::ptr::null_mut(),
                )
            };
            (!hwnd.is_null()).then_some(Self(hwnd))
        }
    }
    impl Drop for Window {
        fn drop(&mut self) {
            unsafe {
                DestroyWindow(self.0);
            }
        }
    }
    struct Lock;
    impl Lock {
        fn open(owner: &Window) -> Option<Self> {
            (unsafe { OpenClipboard(owner.0) } != 0).then_some(Self)
        }
    }
    impl Drop for Lock {
        fn drop(&mut self) {
            unsafe {
                CloseClipboard();
            }
        }
    }
    fn version() -> u32 {
        unsafe { GetClipboardSequenceNumber() }
    }
    fn input_time() -> Option<u32> {
        let mut info = LastInput {
            size: std::mem::size_of::<LastInput>() as u32,
            time: 0,
        };
        (unsafe { GetLastInputInfo(&mut info) } != 0).then_some(info.time)
    }
    fn snapshot() -> Option<Vec<(u32, Vec<u8>)>> {
        let mut data = Vec::new();
        let mut format = 0;
        let mut total = 0;
        loop {
            unsafe {
                SetLastError(0);
            }
            format = unsafe { EnumClipboardFormats(format) };
            if format == 0 {
                if unsafe { GetLastError() } != 0 {
                    return None;
                }
                break;
            }
            // Text, locale, file drop and registered HTML/RTF formats use HGLOBAL.
            if !matches!(format, 1 | 7 | 13 | 15 | 16 | 0xC000..=0xFFFF) {
                return None;
            }
            let handle = unsafe { GetClipboardData(format) };
            let size = unsafe { GlobalSize(handle) };
            total += size;
            if size == 0 || total > 16 * 1024 * 1024 {
                return None;
            }
            let ptr = unsafe { GlobalLock(handle) };
            if ptr.is_null() {
                return None;
            }
            let bytes = unsafe { std::slice::from_raw_parts(ptr, size) }.to_vec();
            unsafe {
                GlobalUnlock(handle);
            }
            data.push((format, bytes));
        }
        Some(data)
    }
    pub fn read() -> Option<String> {
        let owner = Window::new()?;
        let lock = Lock::open(&owner)?;
        let saved = snapshot()?;
        let before = version();
        drop(lock);
        let copy_stamp = crate::core::keystroke::copy_selection().ok()??;
        let deadline = Instant::now() + Duration::from_millis(250);
        while version() == before {
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let copied = version();
        if input_time()? != copy_stamp {
            return None;
        }
        std::thread::sleep(Duration::from_millis(30));
        let _lock = Lock::open(&owner)?;
        if version() != copied || input_time()? != copy_stamp {
            return None;
        }
        let text = snapshot().and_then(|data| {
            let bytes = &data.iter().find(|(format, _)| *format == 13)?.1;
            let utf16: Vec<u16> = bytes
                .as_chunks::<2>()
                .0
                .iter()
                .map(|b| u16::from_le_bytes(*b))
                .take_while(|v| *v != 0)
                .collect();
            Some(String::from_utf16_lossy(&utf16))
        });
        // Allocate everything before emptying, so allocation failure preserves Copy.
        let mut allocated = Vec::new();
        for (format, bytes) in saved {
            let memory = unsafe { GlobalAlloc(2, bytes.len()) };
            let ptr = unsafe { GlobalLock(memory) };
            if ptr.is_null() {
                if !memory.is_null() {
                    unsafe {
                        GlobalFree(memory);
                    }
                }
                for (_, handle) in allocated {
                    unsafe {
                        GlobalFree(handle);
                    }
                }
                return None;
            }
            unsafe {
                std::ptr::copy_nonoverlapping(bytes.as_ptr(), ptr, bytes.len());
                GlobalUnlock(memory);
            }
            allocated.push((format, memory));
        }
        let emptied = unsafe { EmptyClipboard() } != 0;
        for (format, memory) in allocated {
            if !emptied || unsafe { SetClipboardData(format, memory) }.is_null() {
                unsafe {
                    GlobalFree(memory);
                }
            }
        }
        text.filter(|s| !s.trim().is_empty())
    }
}

#[cfg(not(any(target_os = "macos", target_os = "windows")))]
mod platform {
    pub fn read() -> Option<String> {
        None
    }
}

#[cfg(any(target_os = "macos", test))]
fn may_restore(before: u64, copied: u64, current: u64, no_user_input: bool) -> bool {
    copied == before.wrapping_add(1) && current == copied && no_user_input
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn concurrent_copy_or_user_input_wins() {
        assert!(may_restore(10, 11, 11, true));
        assert!(!may_restore(10, 11, 12, true));
        assert!(!may_restore(10, 12, 12, true));
        assert!(!may_restore(10, 11, 11, false));
        assert!(!may_restore(10, 10, 10, true));
    }
}
