use crate::{
    core::system::{self, SystemInfo, SystemResource, UpdateInfo},
    error::LenzError,
};
use tauri::AppHandle;

#[tauri::command]
#[specta::specta]
pub fn get_system_info(app: AppHandle) -> SystemInfo {
    system::info(&app)
}

#[tauri::command]
#[specta::specta]
pub fn open_system_resource(app: AppHandle, resource: SystemResource) -> Result<(), LenzError> {
    system::open_resource(&app, resource)
}

#[tauri::command]
#[specta::specta]
pub async fn check_for_update(app: AppHandle) -> Result<UpdateInfo, LenzError> {
    system::check_update(&app).await
}

#[tauri::command]
#[specta::specta]
pub fn get_update_state(app: AppHandle) -> Option<UpdateInfo> {
    use tauri::Manager;
    app.state::<system::UpdateState>()
        .0
        .lock()
        .ok()
        .and_then(|s| s.clone())
}

#[tauri::command]
#[specta::specta]
pub fn get_license_notices() -> String {
    include_str!("../../resources/LICENSES.txt").into()
}
