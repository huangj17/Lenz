# Spec 11 · 非功能需求

> 覆盖 PRD §5(性能、隐私、兼容性)。补充打包签名与 CI 流程约定。

---

## 1. 性能指标

| 指标 | 目标 | 测量方法 |
|---|---|---|
| 应用启动 → 托盘可用 | ≤ 2s | 冷启动计时(从进程创建到托盘图标可点击) |
| 空闲内存占用 | ≤ 150MB | `Activity Monitor` / `Task Manager`,启动 60s 后读数 |
| 划词触发 → popup 出现(不含翻译) | ≤ 500ms | 从 mouseUp 到 popup `show()` |
| 截图到 OCR 完成 | ≤ 1s(1920×1080 区域) | `cost_ms.ocr` 字段记录 |
| 截图翻译全流程 | ≤ 3s(本地 OCR + 本地 LLM) | `cost_ms` 各段加总 |
| 原图覆盖翻译全流程 | ≤ 4s | 同上 |
| 安装包大小 | ≤ 15MB | CI artifact 检查 |

### 1.1 测量基准环境

- CPU:Apple M 系列 / Intel Core i7(均做测试)。
- 内存:16GB。
- 本地 LLM:LM Studio,4B 量化模型(本地 LLM 耗时本身不在 Lenz 控制范围内,仅测网络 + 处理开销)。

### 1.2 优化策略

- OCR 调用放 `tokio::task::spawn_blocking`,不阻塞 async runtime。
- popup 窗口常驻隐藏而非每次创建,避免窗口创建耗时。
- 配置在内存中缓存(`RwLock<Config>`),不每次读文件。
- 前端 bundle 用 Vite 生产构建:`minify: true`、`treeshake: true`、按需加载 shadcn/ui 组件。
- Rust release build:`opt-level = "s"`(size 优先,控制包体)。

## 2. 隐私要求

| 数据类型 | 处理方式 |
|---|---|
| 截图位图 | 仅在内存中处理,不落盘,处理完立即释放 |
| OCR 识别文字 | 仅传给翻译引擎(用户配置的 API);不缓存到磁盘 |
| 历史记录文本 | 落盘 `history.json`,但用户可一键清空 |
| API Key | 明文存 `config.json`,文件权限 600(macOS/Linux);Windows 无额外保护(MVP 阶段) |
| 崩溃日志 | 仅本地文件,不上报 |
| 遥测/埋点 | **不做任何上报** |

### 2.1 网络请求白名单

应用主动发出的网络请求限于:

1. 翻译 API 调用(用户配置的 URL)。
2. 检查更新(GitHub Releases API):仅当 `config.app.checkUpdate = true`。

不向 Anthropic 或任何第三方发送使用数据。

### 2.2 文件系统访问

Tauri 前端沙箱限制:只允许访问 `app_config_dir` 和 `app_log_dir`。其他路径无权限。

## 3. 兼容性

| 平台 | 最低版本 | 原因 |
|---|---|---|
| macOS | 12.0 (Monterey) | `VNRecognizeTextRequest` 语言检测 API 要求 |
| Windows | 10 1809 (Build 17763) | `Windows.Media.Ocr` API 稳定版本 |
| 架构 | x64 + ARM64 | Apple Silicon 原生;Windows ARM64 后续验证 |

### 3.1 分发产物

| 平台 | 格式 |
|---|---|
| macOS | `.dmg`(含 `.app` bundle) |
| Windows | `.msi`(NSIS installer) |

两平台均由 CI 自动构建。

## 4. 打包与签名

### 4.1 macOS

- 开发者证书:需要 Apple Developer Program 账号。
- 签名:`codesign --deep --force --verify --sign "Developer ID Application: ..."`.
- 公证(Notarization):提交 Apple Notary Service,等待通过后 `xcrun stapler staple`.
- `LSUIElement = true`(Info.plist):不在 Dock 显示,仅驻菜单栏。
- 权限声明(entitlements):
  - `com.apple.security.automation.apple-events`(CGEventTap)
  - `com.apple.security.device.camera`(不需要)
  - 屏幕录制:通过 `NSScreenCaptureUsageDescription`。
  - 辅助功能:通过 `NSAppleEventsUsageDescription`。

### 4.2 Windows

- 代码签名:EV 证书(推荐)或标准 Authenticode,通过 SignTool 签名。
- SmartScreen:有证书后多次发布逐步建立信任度。
- MVP 阶段:可先不签名发布(用户需手动绕过 SmartScreen 警告),在 M9 明确签名策略。

### 4.3 CI 流程

```yaml
# .github/workflows/release.yml (伪代码)
on:
  push:
    tags: ['v*']
jobs:
  build-macos:
    runs-on: macos-latest
    steps:
      - setup Rust + Node
      - pnpm install
      - cargo tauri build --target universal-apple-darwin
      - codesign + notarize
      - upload .dmg artifact
  build-windows:
    runs-on: windows-latest
    steps:
      - setup Rust + Node
      - pnpm install
      - cargo tauri build
      - signtool (if cert available)
      - upload .msi artifact
  release:
    needs: [build-macos, build-windows]
    steps:
      - create GitHub Release
      - upload artifacts
      - generate latest.json for updater
```

## 5. 日志

- 使用 `log` + `tauri-plugin-log`。
- 级别:生产构建 `info` 及以上;debug 构建 `debug` 及以上。
- 日志文件位置:
  - macOS:`~/Library/Logs/app.lenz.desktop/lenz.log`
  - Windows:`%APPDATA%/Lenz/logs/lenz.log`
- 日志轮转:单文件最大 5MB,保留最近 3 个文件。
- 禁止在日志中记录 API Key、用户剪贴板内容。

## 6. 错误上报(MVP 不做)

- MVP 阶段:崩溃/错误仅记录本地日志,用户反馈时手动提供。
- P2:考虑引入本地崩溃导出工具(不自动上报)。

## 7. 验收标准

- [ ] 冷启动 ≤ 2s(M1 Mac / i7 Windows 各测 3 次取平均)。
- [ ] 空闲内存 ≤ 150MB(启动 60s 后,无任何翻译操作)。
- [ ] macOS 安装包通过 Gatekeeper(公证成功)。
- [ ] Windows 安装包无运行时错误(SmartScreen 可有警告,但可绕过)。
- [ ] `.dmg` / `.msi` 文件大小 ≤ 15MB。
- [ ] 抓包确认:无非预期的网络请求(只有翻译 API 和 GitHub 更新请求)。
