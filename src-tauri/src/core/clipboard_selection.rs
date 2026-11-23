//! Clipboard fallback is a single, bounded transaction. Never write a sentinel.
//! Snapshot every native representation before synthesizing Copy; restore it whenever
//! the clipboard still holds exactly what our Copy produced. A concurrent writer wins,
//! and user input only invalidates the text read during the probe: input that cannot
//! write the clipboard (pointer motion, releasing the trigger keys) must never leave
//! the user's clipboard replaced by the selection.
use std::sync::Mutex;
use std::time::Duration;
#[cfg(any(target_os = "macos", target_os = "windows"))]
use std::time::Instant;

static PROBE: Mutex<()> = Mutex::new(());

/// Poll interval while waiting for the synthesized Copy to land.
#[cfg(any(target_os = "macos", target_os = "windows"))]
const POLL: Duration = Duration::from_millis(10);

pub fn read(source: u64) -> Option<String> {
    // Drop overlapping shortcuts instead of queueing copies into a different app.
    let _guard = PROBE.try_lock().ok()?;
    if crate::core::selection_text::foreground_id() != Some(source) {
        return None;
    }
    platform::read(source)
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

    const DEADLINE: Duration = Duration::from_millis(1500);
    const SETTLE: Duration = Duration::from_millis(80);

    fn inputs() -> [u32; 3] {
        [
            CGEventType::KeyDown,
            CGEventType::LeftMouseDown,
            CGEventType::RightMouseDown,
        ]
        .map(|kind| {
            CGEventSource::counter_for_event_type(CGEventSourceStateID::HIDSystemState, kind)
        })
    }

    pub fn read(source: u64) -> Option<String> {
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
            if board.changeCount() != before
                || crate::core::selection_text::foreground_id() != Some(source)
                || crate::core::keystroke::modifiers_held()
            {
                return None;
            }
            let input_before = inputs();
            crate::core::keystroke::copy_selection().ok()?;
            let started = Instant::now();
            // Rich text applications may clear, then publish multiple formats.
            let mut settle = super::Settle::new(before as u64, Duration::ZERO);
            // Synthetic Copy uses a private source at the session tap, so hardware
            // counters only move for deliberate user presses (never pointer motion).
            let mut landed_cleanly = false;
            let mut quiet = true;
            loop {
                let was_landed = settle.landed();
                if settle.observe(board.changeCount() as u64, started.elapsed(), SETTLE) {
                    break;
                }
                if !was_landed && settle.landed() {
                    // A press before the board changed may be the user's own Copy:
                    // the change cannot be attributed to us, so it is never undone.
                    landed_cleanly = inputs() == input_before;
                }
                if inputs() != input_before
                    || crate::core::selection_text::foreground_id() != Some(source)
                {
                    if !settle.landed() {
                        return None;
                    }
                    // Our Copy already replaced the board: keep waiting so it can be
                    // restored, but the selection may have changed under us.
                    quiet = false;
                }
                if settle.expired(started.elapsed(), DEADLINE) {
                    // Unchanged (nothing selected), or another writer keeps writing.
                    return None;
                }
                std::thread::sleep(POLL);
            }
            let text = board
                .stringForType(unsafe { NSPasteboardTypeString })
                .map(|s| s.to_string());
            let quiet = quiet
                && inputs() == input_before
                && crate::core::selection_text::foreground_id() == Some(source);
            let verdict = super::verdict(
                before as u64,
                settle.latest(),
                board.changeCount() as u64,
                landed_cleanly,
                quiet,
            );
            if !verdict.restore {
                return None;
            }
            board.clearContents();
            if !snapshot.is_empty() && !board.writeObjects(&NSArray::from_retained_slice(&snapshot))
            {
                log::warn!("could not restore native clipboard snapshot");
            }
            text.filter(|s| verdict.use_text && !s.trim().is_empty())
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
    /// Word/Excel with large selections or busy Electron apps can take several
    /// hundred milliseconds to publish a copy.
    const DEADLINE: Duration = Duration::from_millis(1000);
    const SETTLE: Duration = Duration::from_millis(50);
    const OPEN_ATTEMPTS: u32 = 20;

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
        /// Clipboard history (Win+V) and managers such as Ditto open the clipboard
        /// the moment it changes; a single attempt would abandon our restore.
        fn open_retry(owner: &Window) -> Option<Self> {
            for _ in 0..OPEN_ATTEMPTS {
                if let Some(lock) = Self::open(owner) {
                    return Some(lock);
                }
                std::thread::sleep(POLL);
            }
            None
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
    pub fn read(_source: u64) -> Option<String> {
        let owner = Window::new()?;
        let (saved, before) = {
            let _lock = Lock::open(&owner)?;
            (snapshot()?, version())
        };
        let copy_stamp = crate::core::keystroke::copy_selection().ok()??;
        let started = Instant::now();
        let mut settle = super::Settle::new(u64::from(before), Duration::ZERO);
        while !settle.observe(u64::from(version()), started.elapsed(), SETTLE) {
            if settle.expired(started.elapsed(), DEADLINE) {
                // Unchanged (nothing selected), or another writer keeps writing.
                return None;
            }
            std::thread::sleep(POLL);
        }
        let Some(_lock) = Lock::open_retry(&owner) else {
            log::warn!("clipboard stayed locked; selection copy could not be restored");
            return None;
        };
        // GetLastInputInfo also counts pointer motion and key releases, so it cannot
        // tell a deliberate user copy from noise. It only invalidates the text; a user
        // copy that lands after ours bumps the sequence number and is never overwritten.
        let quiet = input_time() == Some(copy_stamp);
        let verdict = super::verdict(
            u64::from(before),
            settle.latest(),
            u64::from(version()),
            true,
            quiet,
        );
        if !verdict.restore {
            return None;
        }
        let text = verdict.use_text.then(snapshot).flatten().and_then(|data| {
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
    pub fn read(_source: u64) -> Option<String> {
        None
    }
}

/// Tracks the native clipboard version after a synthesized Copy. One Copy can clear
/// and write the board several times (plain text + rich formats), so the copy is
/// only considered done once the version has changed and then held still for the
/// settle period, rather than assuming it increases exactly once.
#[cfg(any(target_os = "macos", target_os = "windows", test))]
const LANDED_GRACE: Duration = Duration::from_millis(500);

#[cfg(any(target_os = "macos", target_os = "windows", test))]
struct Settle {
    before: u64,
    latest: u64,
    changed_at: Duration,
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
impl Settle {
    fn new(before: u64, now: Duration) -> Self {
        Self {
            before,
            latest: before,
            changed_at: now,
        }
    }

    /// Records one poll; returns true once the copy has landed and settled.
    fn observe(&mut self, version: u64, now: Duration, settle: Duration) -> bool {
        if version != self.latest {
            self.latest = version;
            self.changed_at = now;
        }
        self.landed() && now.saturating_sub(self.changed_at) >= settle
    }

    fn landed(&self) -> bool {
        self.latest != self.before
    }

    /// Gives up when nothing landed by `deadline`. A copy that landed close to the
    /// deadline still gets time to settle so it can be restored; only a board that
    /// keeps changing past the grace period (another writer) is abandoned.
    fn expired(&self, now: Duration, deadline: Duration) -> bool {
        now >= deadline && (!self.landed() || now >= deadline + LANDED_GRACE)
    }

    fn latest(&self) -> u64 {
        self.latest
    }
}

#[cfg(any(target_os = "macos", target_os = "windows", test))]
#[derive(Debug, PartialEq, Eq)]
struct Verdict {
    /// Put the user's snapshot back.
    restore: bool,
    /// The copied text is the current selection and may be translated.
    use_text: bool,
}

/// Decides how a probe ends once the copy has settled.
///
/// - `landed_cleanly`: nothing that could itself copy happened before the board
///   changed, so the change is ours to undo.
/// - `quiet`: no user input or focus change during the whole probe, so the text is
///   still the selection the shortcut was pressed for.
///
/// Restoring only requires that the board still holds the settled version: user
/// input that does not write the clipboard must never cost the user their clipboard.
#[cfg(any(target_os = "macos", target_os = "windows", test))]
fn verdict(before: u64, copied: u64, current: u64, landed_cleanly: bool, quiet: bool) -> Verdict {
    let ours = copied != before && current == copied && landed_cleanly;
    Verdict {
        restore: ours,
        use_text: ours && quiet,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn concurrent_writer_wins() {
        // Another write after ours settled: never overwrite it.
        assert_eq!(
            verdict(10, 11, 12, true, true),
            Verdict {
                restore: false,
                use_text: false
            }
        );
        // Nothing landed: the clipboard was never touched.
        assert!(!verdict(10, 10, 10, true, true).restore);
        // Press before the change landed may be the user's own copy.
        assert!(!verdict(10, 11, 11, false, true).restore);
    }

    #[test]
    fn user_input_after_landing_still_restores_clipboard() {
        // Pointer motion / releasing the trigger keys after our copy landed must
        // not leave the selection on the user's clipboard.
        assert_eq!(
            verdict(10, 11, 11, true, false),
            Verdict {
                restore: true,
                use_text: false
            }
        );
        assert_eq!(
            verdict(10, 12, 12, true, true),
            Verdict {
                restore: true,
                use_text: true
            }
        );
    }

    #[test]
    fn copy_settles_after_last_change() {
        let settle = ms(50);
        let mut s = Settle::new(7, ms(0));
        assert!(!s.observe(7, ms(10), settle));
        assert!(!s.landed());
        // Rich-text apps publish several formats: each write re-arms the settle timer.
        assert!(!s.observe(8, ms(20), settle));
        assert!(!s.observe(9, ms(40), settle));
        assert!(!s.observe(9, ms(80), settle));
        assert!(s.observe(9, ms(90), settle));
        assert_eq!(s.latest(), 9);
    }

    #[test]
    fn unchanged_clipboard_never_settles() {
        let mut s = Settle::new(3, ms(0));
        assert!(!s.observe(3, ms(5_000), ms(50)));
        assert!(!s.landed());
        assert!(s.expired(ms(1_000), ms(1_000)));
    }

    #[test]
    fn late_copy_gets_time_to_settle_before_expiry() {
        let mut s = Settle::new(3, ms(0));
        assert!(!s.observe(4, ms(990), ms(50)));
        // Landed 10ms before the deadline: keep waiting so it can be restored.
        assert!(!s.expired(ms(1_000), ms(1_000)));
        assert!(s.observe(4, ms(1_040), ms(50)));
        // A board that never stops changing is eventually abandoned.
        let mut busy = Settle::new(3, ms(0));
        busy.observe(4, ms(990), ms(50));
        assert!(busy.expired(ms(1_000) + LANDED_GRACE, ms(1_000)));
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires the tests/fixtures/selection.txt opened in TextEdit in the foreground and Accessibility permission; preserves the clipboard"]
    fn native_fixture_copy_returns_selection_and_restores_clipboard() {
        crate::core::selection_text::tests::fixture_selection();
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString};
        objc2::rc::autoreleasepool(|_| {
            let board = NSPasteboard::generalPasteboard();
            let before = board.stringForType(unsafe { NSPasteboardTypeString });
            let source = crate::core::selection_text::foreground_id().expect("fixture source");
            let selected = read(source).expect("copy must return the current fixture selection");
            assert!(selected == include_str!("../../../tests/fixtures/selection.txt"));
            assert!(board.stringForType(unsafe { NSPasteboardTypeString }) == before);
        });
    }

    #[cfg(target_os = "macos")]
    #[test]
    #[ignore = "requires Sublime Text in the foreground with only tests/fixtures/selection.txt text selected and Accessibility permission; preserves the clipboard"]
    fn native_fixture_copy_from_app_without_accessibility_text() {
        use objc2_app_kit::{NSPasteboard, NSPasteboardTypeString, NSWorkspace};
        objc2::rc::autoreleasepool(|_| {
            let foreground = NSWorkspace::sharedWorkspace()
                .frontmostApplication()
                .and_then(|app| app.bundleIdentifier())
                .map(|id| id.to_string());
            assert_eq!(
                foreground.as_deref(),
                Some("com.sublimetext.4"),
                "activate the controlled Sublime Text fixture before running this test"
            );
            assert!(
                crate::core::selection_text::read_selected_text().is_none(),
                "fixture must exercise clipboard fallback rather than AX text"
            );
            let board = NSPasteboard::generalPasteboard();
            let before = board.stringForType(unsafe { NSPasteboardTypeString });
            let source = crate::core::selection_text::foreground_id().expect("fixture source");
            let selected = read(source).expect("copy fallback must read the controlled fixture");
            assert!(selected == include_str!("../../../tests/fixtures/selection.txt"));
            assert!(board.stringForType(unsafe { NSPasteboardTypeString }) == before);
        });
    }
}
