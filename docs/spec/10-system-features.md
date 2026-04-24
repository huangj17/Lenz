# Spec 10 · 系统功能

> 覆盖 PRD §2.3(状态栏图标、自动启动、检查更新、代理设置)与 §4(快捷键)。

---

## 1. 托盘图标

### 1.1 图标

- 文件:`src-tauri/icons/tray.png`(macOS:16×16 & 32×32,Windows:16×16 & 32×32)。
- macOS:模板图标(黑白,系统自动适配深浅色)。
- Windows:彩色图标。

### 1.2 托盘菜单

```
打开主窗口
───────────
划词翻译       Alt+D
截图翻译       Alt+S
原图覆盖翻译   Alt+F
───────────
[✓] 划词监听(勾选表示已启用)
───────────
设置...
───────────
检查更新
关于 Lenz
───────────
退出
```

- "划词监听"是 toggle:点击调 `toggle_selection_listener`,更新 `config.selection.enabled`,菜单项勾选状态同步。
- 快捷键显示:从 `config.shortcuts` 读取,若为 null 则不显示。

### 1.3 实现

```rust
// setup.rs
fn build_tray(app: &AppHandle) -> Result<TrayIcon, LenzError> {
    let menu = build_tray_menu(app);
    TrayIconBuilder::new()
        .icon(app.default_window_icon().unwrap().clone())
        .menu(&menu)
        .on_menu_event(handle_menu_event)
        .on_tray_icon_event(|tray, event| {
            if let TrayIconEvent::Click { button: MouseButton::Left, .. } = event {
                show_main_window(tray.app_handle());
            }
        })
        .build(app)?
}
```

配置变更后(快捷键/划词开关)重建菜单:`rebuild_tray_menu(app)`。

## 2. 全局快捷键

### 2.1 注册

```rust
fn register_shortcuts(app: &AppHandle, shortcuts: &ShortcutConfig) -> Result<(), LenzError> {
    let manager = app.global_shortcut();
    manager.unregister_all()?;
    register_if_set(&manager, &shortcuts.selection, "shortcut://selection", app);
    register_if_set(&manager, &shortcuts.screenshot, "shortcut://screenshot", app);
    register_if_set(&manager, &shortcuts.overlay, "shortcut://overlay", app);
    register_if_set(&manager, &shortcuts.open_main, "shortcut://open-main", app);
    Ok(())
}
```

`register_if_set`:若快捷键字符串为空/null 则跳过。

### 2.2 快捷键字符串格式

采用 Tauri 全局快捷键格式(基于 `tao`/`muda`):
- 修饰键:`Ctrl`、`Alt`、`Shift`、`Super`(Win/Cmd)。
- macOS 专用:`Option`(≡`Alt`)。
- 示例:`"Alt+D"`,`"Option+S"`,`"Ctrl+Alt+T"`。

前端快捷键录制时将 `KeyboardEvent` 转换为此格式,Rust 侧直接使用。

### 2.3 冲突检测

- 同一应用内:注册前检查已有快捷键 → UI 显示警告。
- 与系统的冲突无法检测;用户自行处理。

### 2.4 配置变更重绑

监听 `config://changed` 包含 `shortcuts.*` 时:调 `register_shortcuts` 重新注册(会先全部注销)。

## 3. 自动启动

使用 `tauri-plugin-autostart`。

```rust
// 读取配置后
if config.app.auto_start {
    app.autolaunch().enable()?;
} else {
    app.autolaunch().disable()?;
}
```

配置变更时(监听 `config://changed` → `app.autoStart`):立即更新注册状态。

- macOS:在 `~/Library/LaunchAgents/` 写入 plist,`LSUIElement = true`。
- Windows:写注册表 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run`。

## 4. 检查更新

使用 `tauri-plugin-updater`。

### 4.1 触发时机

- 启动后 3s 延迟(避免阻塞启动流程):若 `config.app.checkUpdate = true`。
- 用户在设置页或托盘点击"检查更新":手动触发。

### 4.2 流程

```rust
async fn check_update(app: AppHandle) -> Result<(), LenzError> {
    let update = app.updater()?.check().await?;
    if let Some(update) = update {
        app.emit("app://update-available", UpdateInfo {
            version: update.version.clone(),
            notes: update.body.clone(),
            download_url: update.download_url.to_string(),
        })?;
    }
}
```

前端监听 `app://update-available` → 弹出提示 dialog:
- 展示新版本号与 changelog。
- 按钮:"下载更新"(在浏览器打开 GitHub Release 页)、"稍后提醒"。
- 不内置自动下载安装(避免签名/权限复杂度)。

### 4.3 版本源

`tauri.conf.json` 中配置 `updater.endpoints`:指向 GitHub Releases 的 `latest.json`(由 CI 自动生成)。

## 5. 代理设置

代理配置在 `config.proxy.*`,详见 [Spec 01 §2](./01-config-storage.md)。

### 5.1 reqwest Client 重建

- `TranslatorManager` 内维护共享 `reqwest::Client`。
- 监听 `config://changed` 包含 `proxy.*` → 重建 client(见 [Spec 04 §7](./04-translator.md))。
- 本地引擎(`useProxy = false`)始终使用无代理 client。

### 5.2 前端设置 UI

通用 Tab → 代理区块:

```
[✓] 启用代理

协议: [HTTP ▾]  地址: [________]  端口: [____]

用户名(可选): [________]   密码(可选): [••••••••]  [显示]

[✓] 本地地址不经代理

[测试连接]
```

"测试连接":向 `https://www.google.com`(或配置的测试 URL)发一个 HEAD 请求,显示延迟或错误。

## 6. 语言与主题

详见 [Spec 09 §6.1~6.4](./09-windows-ui.md)。此处补充系统级行为:

- 系统主题变化:Rust 侧通过 `tauri::window::Window::theme()` 监听,emit `config://theme-changed { theme: "light"|"dark" }`。
- 系统语言变化:不实时监听;用户需手动重启或在设置页切换。

## 7. 关于页

- 应用版本:`app.package_info().version`。
- 构建日期:`env!("BUILD_DATE")`(CI 注入)。
- 开源协议:MIT,展示全文链接。
- GitHub 仓库链接。
- 第三方许可证:链接到打包目录内的 `LICENSES.html`(由 `cargo-about` 生成)。

## 8. 验收标准

- [ ] 托盘图标在 macOS 深/浅色模式下均清晰。
- [ ] 托盘菜单"划词监听"toggle 与实际监听状态同步。
- [ ] 快捷键修改后立即生效,不需重启。
- [ ] 自启开关修改后,重启系统验证是否自动运行。
- [ ] 检查更新:有新版本时弹框;无新版本时 toast "已是最新版"。
- [ ] 代理启用后,翻译 API 请求经过代理(用 Charles/mitmproxy 验证)。

## 9. 开放问题

- [ ] macOS 未公证(Gatekeeper)时自启 plist 是否能正常工作——M1 末尾做完整验证。
- [ ] 自动更新是否未来升级为应用内下载安装(需要签名证书),MVP 阶段先用浏览器打开。
