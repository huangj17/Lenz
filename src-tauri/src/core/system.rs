use std::sync::{Arc, Mutex};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use specta::Type;
use tauri::{AppHandle, Emitter, Manager};

use crate::config::{schema::Locale, ConfigStore};
use crate::core::{ocr::OcrEngine, selection, translator::TranslatorManager};
use crate::error::LenzError;

pub const REPOSITORY_URL: &str = "https://github.com/huangj17/Lenz";
pub const RELEASES_URL: &str = "https://github.com/huangj17/Lenz/releases";
const RELEASE_API: &str = "https://api.github.com/repos/huangj17/Lenz/releases/latest";

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SystemInfo {
    pub platform: String,
    pub version: String,
    pub identifier: String,
    pub build_date: String,
    pub repository_url: String,
    pub releases_url: String,
    pub ocr_backend: String,
    pub ocr_languages: Vec<String>,
    pub windows_package_identity: Option<String>,
    pub ocr_fast_mode_supported: bool,
    pub ocr_confidence_supported: bool,
    pub ocr_auto_detect_supported: bool,
    pub accessibility_granted: bool,
    pub screen_recording_granted: bool,
    pub selection_running: bool,
    pub apple_translation_available: bool,
    pub active_engine_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UpdateInfo {
    pub current_version: String,
    pub latest_version: Option<String>,
    pub available: bool,
    pub release_url: String,
    pub notes: String,
    pub published: bool,
}

#[derive(Default)]
pub struct UpdateState(pub Mutex<Option<UpdateInfo>>, pub tokio::sync::Mutex<()>);

pub fn is_chinese<R: tauri::Runtime>(app: &AppHandle<R>) -> bool {
    let locale = app
        .state::<ConfigStore>()
        .get()
        .map(|c| c.app.locale)
        .unwrap_or(Locale::System);
    match locale {
        Locale::ZhCn => true,
        Locale::EnUs => false,
        Locale::System => {
            #[cfg(target_os = "macos")]
            let tag = objc2_foundation::NSLocale::preferredLanguages()
                .firstObject()
                .map(|tag| tag.to_string());
            #[cfg(target_os = "windows")]
            let tag = {
                use windows::Win32::Globalization::GetUserDefaultLocaleName;
                let mut buffer = [0u16; 85];
                let n = unsafe { GetUserDefaultLocaleName(&mut buffer) };
                (n > 1).then(|| String::from_utf16_lossy(&buffer[..n as usize - 1]))
            };
            #[cfg(not(any(target_os = "macos", target_os = "windows")))]
            let tag = std::env::var("LANG").ok();
            tag.map(|t| t.trim().to_ascii_lowercase().starts_with("zh"))
                .unwrap_or(false)
        }
    }
}

pub fn info(app: &AppHandle) -> SystemInfo {
    let ocr = app.state::<Arc<dyn OcrEngine>>();
    SystemInfo {
        platform: std::env::consts::OS.into(),
        version: app.package_info().version.to_string(),
        identifier: app.config().identifier.clone(),
        build_date: env!("LENZ_BUILD_DATE").into(),
        repository_url: REPOSITORY_URL.into(),
        releases_url: RELEASES_URL.into(),
        ocr_backend: ocr.name().into(),
        ocr_languages: ocr.supported_languages(),
        windows_package_identity: {
            #[cfg(target_os = "windows")]
            {
                crate::core::windows_package::full_name().ok().flatten()
            }
            #[cfg(not(target_os = "windows"))]
            {
                None
            }
        },
        ocr_fast_mode_supported: cfg!(target_os = "macos"),
        ocr_confidence_supported: cfg!(target_os = "macos"),
        ocr_auto_detect_supported: {
            #[cfg(target_os = "macos")]
            {
                crate::core::ocr::auto_detect_supported()
            }
            #[cfg(not(target_os = "macos"))]
            {
                true
            }
        },
        accessibility_granted: selection::permission_granted(),
        screen_recording_granted: screen_recording_granted(),
        selection_running: app.state::<selection::SelectionListener>().is_running(),
        apple_translation_available: crate::core::translator::apple_available(),
        active_engine_id: app
            .state::<TranslatorManager>()
            .get(None)
            .map(|e| e.id().into()),
    }
}

fn screen_recording_granted() -> bool {
    #[cfg(target_os = "macos")]
    {
        #[link(name = "CoreGraphics", kind = "framework")]
        extern "C" {
            fn CGPreflightScreenCaptureAccess() -> bool;
        }
        unsafe { CGPreflightScreenCaptureAccess() }
    }
    #[cfg(not(target_os = "macos"))]
    {
        true
    }
}

#[derive(Deserialize)]
struct Release {
    tag_name: String,
    body: Option<String>,
    html_url: String,
    draft: bool,
    prerelease: bool,
}

fn release_info(current: &str, release: Release) -> Result<UpdateInfo, LenzError> {
    let version = release.tag_name.trim_start_matches('v');
    let latest = semver::Version::parse(version)
        .map_err(|_| LenzError::Network("invalid release version".into()))?;
    let current_version =
        semver::Version::parse(current).map_err(|e| LenzError::internal(e.to_string()))?;
    // Only official HTTPS release links are ever offered for opening.
    if !release
        .html_url
        .starts_with(&format!("{RELEASES_URL}/tag/"))
    {
        return Err(LenzError::Network("invalid release URL".into()));
    }
    Ok(UpdateInfo {
        current_version: current.into(),
        latest_version: Some(latest.to_string()),
        available: !release.draft && !release.prerelease && latest > current_version,
        release_url: release.html_url,
        notes: release.body.unwrap_or_default(),
        published: true,
    })
}

pub async fn check_update(app: &AppHandle) -> Result<UpdateInfo, LenzError> {
    let state = app.state::<UpdateState>();
    let _guard = state.1.lock().await;
    let current = app.package_info().version.to_string();
    let response = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(format!("Lenz/{current}"))
        .build()
        .map_err(|e| LenzError::Network(e.to_string()))?
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|e| LenzError::Network(e.to_string()))?;
    let result = if response.status() == reqwest::StatusCode::NOT_FOUND {
        UpdateInfo {
            current_version: current,
            latest_version: None,
            available: false,
            release_url: RELEASES_URL.into(),
            notes: String::new(),
            published: false,
        }
    } else {
        let release: Release = response
            .error_for_status()
            .map_err(|e| LenzError::Network(e.to_string()))?
            .json()
            .await
            .map_err(|e| LenzError::Network(e.to_string()))?;
        release_info(&current, release)?
    };
    let notify = result.available
        && state
            .0
            .lock()
            .ok()
            .and_then(|s| {
                s.as_ref()
                    .map(|u| u.latest_version != result.latest_version)
            })
            .unwrap_or(true);
    *state
        .0
        .lock()
        .map_err(|_| LenzError::internal("update lock poisoned"))? = Some(result.clone());
    app.emit("app://update-checked", &result)?;
    if notify {
        crate::tray::show_main_window(app);
    }
    Ok(result)
}

pub fn open_url(url: &str) -> Result<(), LenzError> {
    #[cfg(target_os = "macos")]
    let status = std::process::Command::new("open").arg(url).status()?;
    #[cfg(target_os = "windows")]
    let status = std::process::Command::new("rundll32.exe")
        .args(["url.dll,FileProtocolHandler", url])
        .status()?;
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let status = std::process::Command::new("xdg-open").arg(url).status()?;
    if status.success() {
        Ok(())
    } else {
        Err(LenzError::internal("could not open resource"))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum SystemResource {
    Repository,
    Releases,
    Accessibility,
    ScreenRecording,
    OcrLanguages,
    LatestRelease,
}

pub fn open_resource(app: &AppHandle, resource: SystemResource) -> Result<(), LenzError> {
    let url = match resource {
        SystemResource::Repository => REPOSITORY_URL.to_string(),
        SystemResource::Releases => RELEASES_URL.to_string(),
        SystemResource::LatestRelease => app
            .state::<UpdateState>()
            .0
            .lock()
            .ok()
            .and_then(|s| s.as_ref().map(|u| u.release_url.clone()))
            .unwrap_or(RELEASES_URL.into()),
        SystemResource::Accessibility => {
            #[cfg(target_os = "macos")]
            {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_Accessibility"
                    .into()
            }
            #[cfg(not(target_os = "macos"))]
            {
                "ms-settings:easeofaccess".into()
            }
        }
        SystemResource::ScreenRecording => {
            #[cfg(target_os = "macos")]
            {
                "x-apple.systempreferences:com.apple.preference.security?Privacy_ScreenCapture"
                    .into()
            }
            #[cfg(not(target_os = "macos"))]
            {
                "ms-settings:privacy".into()
            }
        }
        SystemResource::OcrLanguages => {
            #[cfg(target_os = "macos")]
            {
                "x-apple.systempreferences:com.apple.Localization-Settings.extension".into()
            }
            #[cfg(not(target_os = "macos"))]
            {
                "ms-settings:regionlanguage".into()
            }
        }
    };
    open_url(&url)
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(tag: &str) -> Release {
        Release {
            tag_name: tag.into(),
            body: None,
            html_url: format!("{RELEASES_URL}/tag/{tag}"),
            draft: false,
            prerelease: false,
        }
    }
    #[test]
    fn update_compares_semantic_versions_and_rejects_external_links() {
        assert!(release_info("0.9.0", release("v0.10.0")).unwrap().available);
        assert!(!release_info("1.0.0", release("v0.10.0")).unwrap().available);
        let mut foreign = release("v1.0.0");
        foreign.html_url = "https://example.com".into();
        assert!(release_info("0.1.0", foreign).is_err());
    }
}
