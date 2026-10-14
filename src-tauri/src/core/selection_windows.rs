use crate::types::SelectionDetected;
use std::{
    cell::RefCell,
    sync::mpsc,
    thread::JoinHandle,
    time::{Duration, Instant},
};
use tauri::{AppHandle, Emitter};
use windows::Win32::{
    Foundation::{LPARAM, LRESULT, WPARAM},
    System::Threading::GetCurrentThreadId,
    UI::WindowsAndMessaging::*,
};
pub struct ListenerHandle {
    thread_id: u32,
    join: JoinHandle<()>,
}
impl ListenerHandle {
    pub fn stop(self) {
        unsafe {
            let _ = PostThreadMessageW(self.thread_id, WM_QUIT, WPARAM(0), LPARAM(0));
        }
        let _ = self.join.join();
    }
}
struct Context {
    app: AppHandle,
    down: Option<(i32, i32)>,
    last_up: Option<(i32, i32, Instant)>,
}
thread_local! { static CONTEXT: RefCell<Option<Context>> = const { RefCell::new(None) }; }
pub fn start(app: &AppHandle) -> Option<ListenerHandle> {
    let app = app.clone();
    let error_app = app.clone();
    let (sender, receiver) = mpsc::sync_channel(1);
    let join = std::thread::Builder::new()
        .name("lenz-selection-listener".into())
        .spawn(move || unsafe {
            let mut message = MSG::default();
            let _ = PeekMessageW(&mut message, None, 0, 0, PM_NOREMOVE);
            let hook = match SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_event), None, 0) {
                Ok(hook) => hook,
                Err(error) => {
                    let _ = sender.send(Err(error.to_string()));
                    return;
                }
            };
            CONTEXT.with(|state| {
                *state.borrow_mut() = Some(Context {
                    app,
                    down: None,
                    last_up: None,
                })
            });
            if sender.send(Ok(GetCurrentThreadId())).is_err() {
                let _ = UnhookWindowsHookEx(hook);
                CONTEXT.with(|state| *state.borrow_mut() = None);
                return;
            }
            while GetMessageW(&mut message, None, 0, 0).0 > 0 {
                let _ = TranslateMessage(&message);
                DispatchMessageW(&message);
            }
            let _ = UnhookWindowsHookEx(hook);
            CONTEXT.with(|state| *state.borrow_mut() = None);
        })
        .ok()?;
    match receiver.recv_timeout(Duration::from_secs(3)) {
        Ok(Ok(thread_id)) => Some(ListenerHandle { thread_id, join }),
        Ok(Err(error)) => {
            let _ = error_app.emit(
                "app://operation-error",
                format!("Windows selection listener: {error}"),
            );
            let _ = join.join();
            None
        }
        Err(_) => None,
    }
}
unsafe extern "system" fn mouse_event(code: i32, event: WPARAM, data: LPARAM) -> LRESULT {
    if code >= 0 && data.0 != 0 && matches!(event.0 as u32, WM_LBUTTONDOWN | WM_LBUTTONUP) {
        let point = unsafe { (*(data.0 as *const MSLLHOOKSTRUCT)).pt };
        CONTEXT.with(|state| {
            let mut state = state.borrow_mut();
            let Some(context) = state.as_mut() else {
                return;
            };
            let (x, y) = crate::core::cursor::physical_to_logical(point);
            if event.0 as u32 == WM_LBUTTONDOWN {
                context.down = Some((point.x, point.y));
                if !crate::core::indicator::hit_test(point.x as f64, point.y as f64) {
                    crate::core::indicator::cancel_pending();
                    let _ = context.app.emit(super::EVENT_SELECTION_DISMISS, ());
                }
            } else {
                let dragged = context
                    .down
                    .take()
                    .is_some_and(|(dx, dy)| (dx - point.x).abs() >= 4 || (dy - point.y).abs() >= 4);
                let double = context.last_up.is_some_and(|(px, py, time)| {
                    (px - point.x).abs() < 5
                        && (py - point.y).abs() < 5
                        && time.elapsed().as_millis()
                            <= unsafe {
                                windows::Win32::UI::Input::KeyboardAndMouse::GetDoubleClickTime()
                            } as u128
                });
                context.last_up = Some((point.x, point.y, Instant::now()));
                if dragged || double {
                    crate::core::indicator::set_physical_anchor(point.x, point.y);
                    let _ = context
                        .app
                        .emit(super::EVENT_SELECTION_DETECTED, SelectionDetected { x, y });
                }
            }
        });
    }
    unsafe { CallNextHookEx(None, code, event, data) }
}
