//! Windows 原生窗口回归:实际调用生产 resize 命令,不能用 MockRuntime 替代窗口边框。

use super::{popup_logical_size, resize_popup, resize_popup_width};
use crate::core::screenshot::SelectionPopupPosition;
use tauri::{LogicalSize, Manager, WebviewUrl};

async fn check_resize(app: &tauri::AppHandle) -> Result<(), String> {
    let popup = app.get_webview_window("popup").ok_or("popup missing")?;
    let scale = popup.scale_factor().map_err(|e| e.to_string())?;
    let tolerance = 1.0 / scale + 0.01; // 允许一次物理像素取整,不允许累计漂移。
    popup.show().map_err(|e| e.to_string())?;
    popup
        .set_size(LogicalSize::new(360.0, 220.0))
        .map_err(|e| e.to_string())?;
    let initial = popup_logical_size(&popup);
    let outer = popup.outer_size().map_err(|e| e.to_string())?;
    println!("popup baseline: scale={scale}, inner={initial:?}, outer={outer:?}");

    // 正向对照:旧实现的 outer -> inner 回填在有外框时会真实放大。
    for _ in 0..8 {
        let outer = popup
            .outer_size()
            .map_err(|e| e.to_string())?
            .to_logical::<f64>(scale);
        popup
            .set_size(LogicalSize::new(outer.width, 220.0))
            .map_err(|e| e.to_string())?;
    }
    let old_width = popup_logical_size(&popup).0;
    println!("old resize path after 8 updates: width={old_width}");
    if outer.width as f64 / scale > initial.0 + tolerance && old_width <= initial.0 + tolerance {
        return Err("positive control did not reproduce frame accumulation".into());
    }

    // 模拟内容变化、流式结果、展开/折叠触发的反复高度测量。
    for width in [300.0, 360.0, 537.0, 720.0] {
        popup
            .set_size(LogicalSize::new(width, 220.0))
            .map_err(|e| e.to_string())?;
        for iteration in 0..100 {
            let height = [96.0, 150.0, 480.0, 220.0][iteration % 4];
            resize_popup(height, app.clone())
                .await
                .map_err(|e| e.to_string())?;
            let actual = popup_logical_size(&popup);
            if (actual.0 - width).abs() > tolerance || (actual.1 - height).abs() > tolerance {
                return Err(format!(
                    "height update {iteration}: expected ({width}, {height}), got {actual:?}"
                ));
            }
        }
    }
    // 右缘手柄调整宽度时也必须保留内容高度,包含上下限裁剪。
    for iteration in 0..100 {
        let requested: f64 = [200.0, 537.0, 900.0, 360.0][iteration % 4];
        resize_popup_width(requested, app.clone())
            .await
            .map_err(|e| e.to_string())?;
        let actual = popup_logical_size(&popup);
        if (actual.0 - requested.clamp(300.0, 720.0)).abs() > tolerance
            || (actual.1 - 220.0).abs() > tolerance
        {
            return Err(format!("width update {iteration}: got {actual:?}"));
        }
    }
    println!("popup resize passed: 400 height updates and 100 width updates without drift");
    Ok(())
}

#[test]
#[ignore = "requires an interactive Windows desktop and WebView2; run explicitly in Windows CI"]
fn native_popup_resize_preserves_client_dimensions() {
    let mut context = tauri::generate_context!();
    let mut popup = context
        .config()
        .app
        .windows
        .iter()
        .find(|window| window.label == "popup")
        .expect("production popup config")
        .clone();
    // 使用生产窗口属性,空页面隔离 React/翻译服务;不加载或修改用户配置。
    popup.url = WebviewUrl::External("about:blank".parse().unwrap());
    context.config_mut().app.windows = vec![popup];
    let (sender, receiver) = std::sync::mpsc::channel();
    let app = tauri::Builder::default()
        .any_thread()
        .manage(SelectionPopupPosition::default())
        .setup(move |app| {
            let app = app.handle().clone();
            tauri::async_runtime::spawn(async move {
                let result =
                    tokio::time::timeout(std::time::Duration::from_secs(60), check_resize(&app))
                        .await
                        .unwrap_or_else(|_| Err("popup resize timed out".into()));
                let _ = sender.send(result);
                app.exit(0);
            });
            Ok(())
        })
        .build(context)
        .expect("build native popup test app");
    app.run_return(|_, _| {});
    receiver
        .recv_timeout(std::time::Duration::from_secs(5))
        .expect("native resize result")
        .expect("native popup dimensions must not drift");
}
