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
| macOS | 12.0 (Monterey) | 自动语言检测需 13+;Apple 翻译需 15+,低版本设置页说明限制 |
| Windows | 10 1809 (Build 17763) | `Windows.Media.Ocr` API 稳定版本 |
| 架构 | x64 + ARM64 | Apple Silicon 原生;Windows ARM64 后续验证 |

### 3.1 分发产物

| 平台 | 格式 |
|---|---|
| macOS | `.dmg`(含 `.app` bundle) |
| Windows | 完整 `.msix` + 安装/识别验收 PowerShell 助手;开发用 exe/MSI 不作为系统 OCR 的正式分发产物 |

Windows 的构建和安装识别工作流已编写,实际运行结果独立验收。macOS 正式签名与发布 CI 尚待配置。

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

- 包身份:`src-tauri/msix/identity.json`;完整包内 `lenz.exe`,`Windows.FullTrustApplication` + `runFullTrust`,支持 Windows 10 1809+。MSIX 名称、Publisher、版本必须在升级中保持兼容;Publisher 必须与签名证书 Subject 一致。默认 Publisher 是待正式证书确定的配置,不能当作已有公开可信签名。
- 构建:`pnpm windows:msix -CertificateThumbprint <thumbprint>`;原生 MSVC、静态 CRT、Tauri `--no-bundle` 后生成清单/图标,Windows SDK MakeAppx 完整校验并打包,SignTool SHA256 签名。正式签名可指定 `-TimestampUrl` 使用证书提供商的 RFC 3161 时间戳服务。x64 默认,`-Architecture arm64` 支持生成 ARM64 包但仍待原生验收。PE 检查拒绝架构不符、遗漏动态 WebView2Loader 和外部 VC++ runtime 依赖。
- WebView2:MSVC Loader 静态链接;若提供已有动态链接 exe,需 `-LoaderPath` 指定匹配架构 DLL。清单声明 `Microsoft.WebView2` ExternalDependency。双击 MSIX 由支持该功能的 Microsoft App Installer 处理;PowerShell 安装助手另行检查三处官方 runtime 注册项,缺少时下载官方 bootstrapper、验证 Microsoft 签名并安装,避免 `Add-AppxPackage` 忽略 ExternalDependency。
- 安装:`Install-Lenz.ps1 -PackagePath <signed.msix>` 由标准 `Add-AppxPackage` 验证包哈希/签名/Publisher/证书信任并安装,随后保证 WebView2 可用,默认从已安装的 AUMID 运行 OCR 诊断。没有注册散装 manifest 或允许 unsigned 的旁路,不自动修改证书信任。识别失败返回错误和诊断路径,包已安装可修复语言能力后重试。`-SkipOcrCheck` 仅用于单独排查安装步骤,不能作为 OCR 验收。开始菜单和包内 StartupTask 保留身份;不要复制 exe 到包外启动。
- 当前用户尚无正式证书。`pnpm windows:msix:test` 默认创建 14 天自签测试证书并导出公开 `.cer`,发布构建用 `-CertificateValidityDays 365`;签完删除构建机私钥,不导出 PFX、不自动修改信任。需明确检查并信任此测试证书才能安装。v0.1.0 按用户要求先公开分发测试签名包并推迟 Windows 运行验收;公开可信签名、稳定生产 Publisher 仍待正式证书/Store 配置。未签名 MSIX 仅能检查内容,不能绕过安装签名要求。
- 详细命令和验收步骤见 [Windows 分发指南](../windows-distribution.md)。官方依据:[完整 MSIX 转换](https://learn.microsoft.com/en-us/windows/msix/desktop/desktop-to-uwp-manual-conversion)、[签名证书](https://learn.microsoft.com/en-us/windows/msix/package/create-certificate-package-signing)、[WebView2 分发](https://learn.microsoft.com/en-us/microsoft-edge/webview2/concepts/distribution)。

### 4.3 CI 流程

`.github/workflows/release.yml` 在 `v*` 标签 push 后校验标签与 Rust/Tauri/npm 三处版本一致,并构建 macOS Apple Silicon/Intel DMG、Windows x64 MSIX。macOS 自动选择 Xcode 26+、最低系统 12、ad-hoc 签名;Windows 使用 365 天自签测试证书。全部构建成功后上传 GitHub Release 及 SHA256 校验值。分支上的手动运行只构建 artifacts,不发布 Release。Developer ID 公证和 Windows 公开可信签名仍待后续配置。

`.github/workflows/windows-msix.yml` 保留为手动验收工作流,默认仅构建;选中 `verify_installed_ocr` 才执行安装识别。v0.1.0 按用户要求暂不运行此验收:

1. `windows-latest` 原生 MSVC 构建并测试签名;上传 `.msix`、公开证书和安装助手,不上传私钥。
2. `windows-11-arm` 客户端下载 x64 包,准备英语 OCR 能力,临时信任仅该次 CI 测试证书,安装包并通过 AUMID 运行合成图片识别。断言实际包身份、文字和 bbox 文本块;输出 `installed-ocr.json`。该路径覆盖 Windows 11 ARM 上的 x64 模拟,不代替 Windows 10/x64 原生与 ARM64 包验收。
3. `finally` 移除本次 CI 专用包及证书。脚本限制为 GitHub-hosted 一次性 runner,拒绝本地或 self-hosted 机器;CI 使用独立名称 `app.lenz.desktop.ci`,不会删除正式包。

手动启用验收后,缺语言能力、签名/安装失败、身份缺失或 OCR 失败都会令测试失败,不静默跳过。Windows 运行验收尚未执行,不能仅凭构建成功宣称 OCR 通过。公开 Release tag 继续作为检查更新版本源(见 spec 10 §4)。

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
- [ ] Windows MSIX 签名可信、WebView2 可用,安装后通过实际 OCR 诊断。
- [ ] Windows 10/x64 原生、Windows 11/ARM64 安装及登录启动后仍有包身份。
- [ ] `.dmg` / `.msix` 文件大小 ≤ 15MB。
- [ ] 抓包确认:无非预期的网络请求(只有翻译 API 和 GitHub 更新请求)。
