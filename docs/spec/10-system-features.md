# Spec 10 · 系统功能

> 覆盖 PRD §2.3(状态栏图标、自动启动、检查更新)与 §4(快捷键)。

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
划词翻译        ⌥D
截图翻译        ⌥S
原图覆盖翻译    ⌥F
───────────
[✓] 划词监听(勾选表示已启用)
───────────
设置...        ⌘, (macOS)
───────────
检查更新
关于 Lenz
───────────
退出
```

- "划词监听"是 toggle:点击调 `toggle_selection_listener`,更新 `config.selection.enabled`,菜单项勾选状态同步。设置页「通用 · 划词」也提供同一开关(「划词显示翻译图标」),与本菜单项共用 `config.selection.enabled`,双向同步(见 spec 09 §2.2、spec 05 §1)。
- 翻译功能快捷键从 `config.shortcuts` 读取,通过原生菜单 accelerator 统一显示在右侧快捷键列,与「设置...」对齐;macOS 使用系统符号(如上例的 `⌥D`、`⌥S`、`⌥F`),Windows 使用对应组合键文字(如 `Alt+D`);若为 null 或空字符串则不显示。配置变更重建菜单后同步更新。
- macOS「设置...」使用固定的原生菜单快捷键 `Command+,`,托盘菜单显示 `⌘,`;同一菜单项也加入默认应用菜单,在 Lenz 窗口获得焦点时显示、取消最小化并聚焦主设置窗口。此快捷键不注册为全局快捷键,与可自定义的 `config.shortcuts.openMain` 独立。
- Windows「设置...」使用固定的 `Ctrl+,`,托盘原生菜单在右侧显示 `Ctrl+,`;各 Lenz 窗口通过前端 `keydown` 监听发出已有的 `shortcut://open-main` 事件,复用后端的显示、取消最小化和聚焦逻辑。仅处理精确的 Ctrl+逗号组合,按字符匹配以兼容键盘布局,忽略长按重复、输入法组合及快捷键录制已消费的事件,卸载时清理监听。此快捷键也不注册为全局快捷键。

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

`set_auto_start` 先更新系统注册项,成功后保存 `app.autoStart`;保存失败回滚系统项。启动及配置热更新时校准注册状态。

- macOS:写用户 `~/Library/LaunchAgents/` plist,指向当前应用路径。
- Windows MSIX:调用清单内 `LenzStartup` 的 `StartupTask.RequestEnableAsync` / `Disable`,保持登录启动的包身份。用户或组策略禁用时返回明确错误,不绕过系统设置。
- Windows 普通 EXE 安装版及无包开发版:使用 `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` 的 Lenz 值;该路径可使用腾讯云等第三方 OCR,不提供 Windows 系统 OCR。
- 无需自启插件;重启系统后自动启动仍单列实机验收。

## 4. 检查更新

### 4.1 触发时机

开启 `app.checkUpdate` 时启动后 3s 首次检查,之后每 24h 检查;运行中重新开启立即检查。设置页和托盘支持手动检查。

### 4.2 流程

`check_for_update` 用 reqwest 请求 GitHub Releases API,15s 超时和 Lenz User-Agent;并发检查串行化。semver 比较当前包版本与正式发布 tag,拒绝外部 Release 链接。结果缓存在内存并发 `app://update-checked`;前端通过 `get_update_state` 恢复结果。

新版本显示主窗口和可关闭的提示条,关于页展示版本、更新说明及「打开发布页」。无新版本显示已是最新版;404 表示尚无公开正式版本,不会误报已是最新版。网络错误可再次手动检查。只在浏览器打开已验证的官方 Release URL,不自动下载安装。

### 4.3 版本源

`https://api.github.com/repos/huangj17/Lenz/releases/latest`。直接读取正式 GitHub Release,无需 updater 插件、签名端点或 `latest.json`。应用内安装更新属后续范围。

## 5. 代理设置(已移除)

> 应用内代理功能已移除:不再提供代理配置(`config.proxy.*`、引擎 `useProxy` 均已删除),设置页也不再有代理区块。**编号 §5 保留不复用**,与 spec 08 的处理方式一致。
>
> 翻译引擎共享一个 `reqwest::Client`,沿用 reqwest 默认行为(尊重系统环境变量代理);如需代理访问云端引擎,由用户在系统/环境层配置。详见 [Spec 04 §7](./04-translator.md)。

## 6. 语言与主题

详见 [Spec 09 §6.1~6.4](./09-windows-ui.md)。此处补充系统级行为:

- 系统主题变化:Rust 侧通过 `tauri::window::Window::theme()` 监听,emit `config://theme-changed { theme: "light"|"dark" }`。
- 系统语言变化:不实时监听;用户需手动重启或在设置页切换。

## 7. 关于页

- 应用版本:`app.package_info().version`。
- 构建日期:`LENZ_BUILD_DATE` 秒级时间戳;build.rs 生成,支持 CI 的 `SOURCE_DATE_EPOCH`。
- 开源协议:MIT,展示全文链接。
- GitHub 仓库链接。
- 实际 Bundle ID 和 OCR 后端由 `get_system_info` 返回。
- MIT 与第三方许可证在关于页按依赖选择全文。`python3 scripts/generate-notices.py` 从解析后的 Rust/pnpm 依赖离线生成嵌入资源 `src-tauri/resources/LICENSES.txt`;依赖变更后重新生成。

## 8. 验收标准

- [ ] 托盘图标在 macOS 深/浅色模式下均清晰。
- [ ] 托盘菜单"划词监听"toggle 与实际监听状态同步。
- [ ] 三项翻译快捷键与「设置...」统一在右侧显示,修改或清空快捷键后提示同步更新。
- [ ] macOS 托盘「设置...」显示 `⌘,`,从 Lenz 窗口按 `Command+,` 可打开主设置窗口;配置变更重建托盘菜单后仍可用。
- [ ] Windows 托盘「设置...」显示 `Ctrl+,`,从 Lenz 窗口按 `Ctrl+,` 可打开或恢复最小化的主设置窗口;录制快捷键时不触发,切换到其它应用后不占用该组合键。
- [ ] 快捷键修改后立即生效,不需重启。
- [ ] 自启开关修改后,重启系统验证是否自动运行。
- [ ] 检查更新:新版本提示条和发布页入口;无新版本显示已是最新版;无正式 Release 显示尚未发布。

## 9. 开放问题

- [ ] macOS 未公证(Gatekeeper)时自启 plist 是否能正常工作——M1 末尾做完整验证。
- [ ] 自动更新是否未来升级为应用内下载安装(需要签名证书),MVP 阶段先用浏览器打开。
