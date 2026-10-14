//! MSIX 身份和登录启动。仅在 Windows 编译。
use crate::error::LenzError;
use windows::{
    core::{HSTRING, PWSTR},
    ApplicationModel::{StartupTask, StartupTaskState},
    Win32::{
        Foundation::{APPMODEL_ERROR_NO_PACKAGE, ERROR_INSUFFICIENT_BUFFER, ERROR_SUCCESS},
        Storage::Packaging::Appx::GetCurrentPackageFullName,
        System::Com::{CoInitializeEx, CoUninitialize, COINIT_MULTITHREADED},
    },
};

pub const STARTUP_TASK_ID: &str = "LenzStartup";

pub fn full_name() -> Result<Option<String>, LenzError> {
    let mut length = 0;
    let status = unsafe { GetCurrentPackageFullName(&mut length, None) };
    if status == APPMODEL_ERROR_NO_PACKAGE {
        return Ok(None);
    }
    if status != ERROR_INSUFFICIENT_BUFFER || length == 0 || length > 65536 {
        return Err(LenzError::internal(format!(
            "query package identity: {}",
            status.0
        )));
    }
    let mut buffer = vec![0u16; length as usize];
    let status =
        unsafe { GetCurrentPackageFullName(&mut length, Some(PWSTR(buffer.as_mut_ptr()))) };
    if status != ERROR_SUCCESS {
        return Err(LenzError::internal(format!(
            "read package identity: {}",
            status.0
        )));
    }
    let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
    String::from_utf16(&buffer[..end])
        .map(Some)
        .map_err(|e| LenzError::internal(e.to_string()))
}

pub fn require_identity() -> Result<String, LenzError> {
    full_name()?.ok_or_else(|| {
        LenzError::OcrIdentityRequired(
            "Install the Windows MSIX version and launch Lenz from the Start menu.".into(),
        )
    })
}

struct Apartment(bool);
impl Drop for Apartment {
    fn drop(&mut self) {
        if self.0 {
            unsafe {
                CoUninitialize();
            }
        }
    }
}
fn initialize_apartment() -> Result<Apartment, LenzError> {
    let status = unsafe { CoInitializeEx(None, COINIT_MULTITHREADED) };
    if status.is_ok() {
        Ok(Apartment(true))
    } else if status.0 == 0x80010106u32 as i32 {
        Ok(Apartment(false))
    } else {
        Err(LenzError::internal(format!("initialize COM: {status:?}")))
    }
}

fn startup_task() -> Result<StartupTask, LenzError> {
    StartupTask::GetAsync(&HSTRING::from(STARTUP_TASK_ID))
        .and_then(|op| op.join())
        .map_err(|e| LenzError::internal(format!("MSIX startup task: {e}")))
}
// WinRT join() blocks without pumping an STA. Keep startup operations on an owned MTA thread,
// including when called during Tauri setup or from a synchronous command on the UI thread.
fn with_startup_task<T: Send + 'static>(
    action: impl FnOnce(StartupTask) -> Result<T, LenzError> + Send + 'static,
) -> Result<T, LenzError> {
    std::thread::Builder::new()
        .name("lenz-startup-task".into())
        .spawn(move || {
            let _apartment = initialize_apartment()?;
            action(startup_task()?)
        })
        .map_err(|e| LenzError::internal(e.to_string()))?
        .join()
        .map_err(|_| LenzError::internal("MSIX startup task worker failed"))?
}
pub fn auto_start_enabled() -> Result<bool, LenzError> {
    with_startup_task(|task| {
        let state = task
            .State()
            .map_err(|e| LenzError::internal(e.to_string()))?;
        Ok(matches!(
            state,
            StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
        ))
    })
}
pub fn set_auto_start(enabled: bool) -> Result<(), LenzError> {
    with_startup_task(move |task| {
        if enabled {
            let state = task
                .RequestEnableAsync()
                .and_then(|op| op.join())
                .map_err(|e| LenzError::internal(e.to_string()))?;
            if !matches!(
                state,
                StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
            ) {
                return Err(LenzError::Config("Enable Lenz in Windows Settings > Apps > Startup; startup is disabled by the user or system policy.".into()));
            }
        } else {
            task.Disable()
                .map_err(|e| LenzError::internal(e.to_string()))?;
            let state = task
                .State()
                .map_err(|e| LenzError::internal(e.to_string()))?;
            if matches!(
                state,
                StartupTaskState::Enabled | StartupTaskState::EnabledByPolicy
            ) {
                return Err(LenzError::Config(
                    "Lenz startup is enabled by system policy.".into(),
                ));
            }
        }
        Ok(())
    })
}
