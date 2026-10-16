# Lenz

> 像镜头一样，一眼看懂世界。

基于 **Tauri 2** 的跨平台（macOS + Windows）截图翻译桌面工具：划词翻译、截图翻译、原图覆盖翻译。开源、本地优先、隐私友好，原生支持 LM Studio / Ollama / 任意 OpenAI 兼容接口。

## 📦 下载与安装

最新版本 [v0.1.2](https://github.com/huangj17/Lenz/releases/tag/v0.1.2)：腾讯云高精度 OCR、macOS 快捷键取词与选区弹窗定位修复、权限提示和渐变应用图标。

| 平台 | 安装包 | 最低系统版本 |
|---|---|---|
| macOS Apple Silicon | [aarch64 DMG](https://github.com/huangj17/Lenz/releases/download/v0.1.2/Lenz_0.1.2_aarch64.dmg) | macOS 12 |
| macOS Intel | [x64 DMG](https://github.com/huangj17/Lenz/releases/download/v0.1.2/Lenz_0.1.2_x64.dmg) | macOS 12 |
| Windows x64（推荐，腾讯云等 OCR） | [普通 EXE 安装包](https://github.com/huangj17/Lenz/releases/download/v0.1.2/Lenz_0.1.2_windows_x64_setup.exe) | Windows 10 1809 |
| Windows x64（可选，系统 OCR） | [MSIX + 安装助手 ZIP](https://github.com/huangj17/Lenz/releases/download/v0.1.2/Lenz_0.1.2_windows_x64_test-signed.zip) | Windows 10 1809 |

macOS 打开 DMG 后将 Lenz 拖入「应用程序」。本版使用 ad-hoc 签名，尚未完成 Developer ID 签名与公证；首次打开可能需要在系统「隐私与安全性」中确认。

Windows 推荐下载普通 **EXE 安装包**,无需手动导入证书。安装后在「设置 → 服务 → OCR」添加腾讯云并设为默认即可,无需本地 OCR 语言包。若缺少 WebView2,安装器会下载并验证微软官方运行环境。

以后需要 Windows 系统 OCR 时,可选安装 **MSIX + 安装助手 ZIP**。此版本需要包身份及对应 OCR 语言包;当前使用 365 天自签测试证书,需用户检查后明确导入「本地计算机 → 受信任人」。仅导入证书不能让普通 EXE 启用系统 OCR。切换前请退出应用,按安装说明区分 EXE 与 MSIX 启动入口。详细步骤见 [Windows 分发指南](docs/windows-distribution.md)。

普通 EXE 于 2026-10-16 替换原 MSIX 外壳安装器。EXE 尚无公开可信签名,Windows 可能显示未知发布者/SmartScreen 提示;Windows 原生安装、升级、卸载和腾讯云真实账号识别仍待实测。

安装产物附带 [SHA256SUMS.txt](https://github.com/huangj17/Lenz/releases/download/v0.1.2/SHA256SUMS.txt)。功能和验证范围见 [发布说明](docs/releases/v0.1.2.md)。

## ✨ 核心功能

| 功能 | 快捷键（默认） | 说明 |
|---|---|---|
| 划词翻译（F1） | `⌥ D` / `Alt+D` | 选中文本 → 快捷键或指示图标触发，光标处弹框显示译文 |
| 截图翻译（F2） | `⌥ S` / `Alt+S` | 框选区域 → 所选 OCR 服务 → 翻译 → 弹框显示原文 + 译文 |
| 原图覆盖翻译（F3） | `⌥ F` / `Alt+F` | 框选区域 → OCR + 翻译 → 译文按原文位置覆盖在屏幕上（核心差异化） |
| 设置 | `⌘ ,` / `Ctrl+,` | 打开并聚焦设置窗口，托盘菜单同步展示快捷键 |

配套能力：菜单栏托盘常驻、翻译引擎配置与连接测试、多服务结果对比与独立折叠、系统/百度/腾讯云/视觉模型 OCR 服务管理、OCR 语言与偏好设置、设置搜索、中英双语界面、深浅色主题、登录启动、GitHub Release 更新检查及离线开源许可证。

### 为什么做 Lenz

Bob 是 macOS 上广受好评的翻译工具，但仅支持 macOS、订阅制收费、对本地 LLM 不够友好。Lenz 聚焦其最核心的翻译场景，做一个**开源（MIT）、跨平台、本地优先**的替代品：

- 🖥 **跨平台**：macOS + Windows 统一体验
- 🔒 **本地优先**：默认使用系统 OCR（macOS Vision / Windows.Media.Ocr），配合本地 LLM 可全程在本机处理。也可添加百度 OCR、腾讯云 OCR 或 OpenAI 兼容视觉模型；第三方 OCR 会将图片发送给所选服务。视觉模型当前仅用于截图翻译，原图覆盖继续使用系统 OCR
- 🪶 **轻量**：Tauri 打包，v0.1.0 安装包约 6～7.5MB

## 🛠 技术栈

- **壳与后端**：Tauri 2 + Rust（截图 `xcap`、OCR 系统桥接、macOS CGEventTap / Windows 鼠标 hook 与 UIA 划词监听、配置原子写）
- **前端**：React 19 + TypeScript + Vite 7 + Tailwind CSS v4 + Zustand + i18next
- **类型契约**：`tauri-specta` 从 Rust 命令导出 `src/lib/bindings.ts`，前后端共享类型

## 🚧 项目状态

v0.1.2 的功能实现与平台运行验收分别记录（详见 [`docs/TODO.md`](docs/TODO.md)）：

| 里程碑 | 当前状态 |
|---|---|
| M1 项目搭建 | 窗口、托盘、快捷键、配置与类型导出已实现 |
| M2 OCR 桥接 | macOS Vision 已识别验证；Windows.Media.Ocr、包身份与安装自检已实现，原生 MSVC 构建通过，安装识别待验收 |
| M3 翻译引擎 | OpenAI 兼容、本地 LLM 与传统 API 已接入；Apple 英中翻译及缺语言包提示已验证，Intel 与旧系统运行待验收 |
| M4 截图翻译 F2 | macOS 开发版全流程已跑通；发布版权限回归与 Windows 运行验收待做 |
| M5 划词翻译 F1 | macOS AX / Windows UIA、剪贴板兜底与指示图标已实现；源应用覆盖与权限验收待做 |
| M6 原图覆盖 F3 | 流程、颜色采样、字体适应、取消和关闭已实现；多屏与原生窗口交互待验收 |
| M7 设置页 | 现有设置控件已接通；macOS 设置快捷键与部分偏好重启保存已验证，系统自启和 Windows 运行验收待做 |
| M8 完善 | 中英界面、主题、自启和更新检查已接通；日志轮转、原生毛玻璃与系统交互验收仍有待办 |
| M9 打包发布 | 双架构 macOS DMG、Windows x64 MSIX、安装助手和 EXE 安装向导已发布；正式可信签名、公证和运行验收待完成 |

本地 macOS 已通过类型检查、37 项前端纯逻辑测试、生产构建和 84 项 Rust 测试。[主分支 CI](https://github.com/huangj17/Lenz/actions/workflows/ci.yml) 执行双平台构建与 Rust fmt/Clippy,macOS 运行 Rust 测试,Windows 编译测试。Windows 安装、OCR、自启和 GUI 验收仍需 Windows 客户端运行报告。

`v*` 标签触发双架构 macOS DMG 和 Windows x64 MSIX、安装助手 ZIP、EXE 安装向导构建，成功后上传 [Releases](https://github.com/huangj17/Lenz/releases)。Windows 安装/OCR 验收工作流仅手动触发，默认只构建。

## 🚀 开发

### 环境要求

- [Rust](https://www.rust-lang.org/tools/install)（stable）
- [Node.js](https://nodejs.org/) 22 + [pnpm](https://pnpm.io/) 10（与 CI 一致）
- Tauri 2 的[平台前置依赖](https://tauri.app/start/prerequisites/)
- macOS 构建需要 Xcode 26+ / Swift 6.2+ SDK，以编译 Apple Translation 桥接；应用仍支持 macOS 12，Apple 翻译仅在 macOS 15+ 可用。

### 常用命令

```bash
pnpm install                                          # 安装依赖
pnpm tauri dev                                        # 启动开发（托盘 app + 主窗口）
pnpm typecheck                                        # 前端类型检查
pnpm test                                             # 前端与打包脚本纯逻辑测试
pnpm build                                            # 前端生产构建（不生成桌面安装包）
cargo check --manifest-path src-tauri/Cargo.toml      # Rust 检查
cargo test --manifest-path src-tauri/Cargo.toml       # 当前平台 Rust 测试
cargo test export_bindings --manifest-path src-tauri/Cargo.toml   # 重新生成 bindings.ts
python3 scripts/generate-notices.py                  # 依赖变更后刷新离线许可证
```

macOS 首次运行划词监听 / 截图相关功能时,需在系统设置中授予**辅助功能**与**屏幕录制**权限。

Windows 开发机可用 `pnpm windows:msix:test` 构建测试签名包，也可直接用 `pnpm windows:exe` 构建普通 EXE 安装包（需 NSIS 3）,或加 `-BinaryPath <lenz.exe>` 复用已有 MSVC 构建；直接运行开发 exe 缺少 MSIX 包身份，不能作为系统 OCR 验收方式。安装助手默认不执行 OCR 自检；严格识别验收通过独立 `Test-InstalledOcr.ps1` 执行，构建环境与安装步骤见 [Windows 分发指南](docs/windows-distribution.md)。

### 目录结构

```
├── src/                  # 前端(React)
│   ├── windows/          # 按 ?window= 路由:main / popup / selection / overlay / indicator
│   ├── components/  hooks/  stores/  lib/  locales/  styles/
├── src-tauri/            # 后端(Rust)
│   └── src/
│       ├── commands/     # Tauri 命令
│       ├── core/         # 截图 / OCR / 翻译 / 划词监听 / 弹框定位等核心逻辑
│       ├── config/       # ConfigStore(原子写 + JSON Merge Patch)
│       ├── tray.rs  setup.rs  types.rs  error.rs
└── docs/                 # 产品与技术文档
    ├── PRD.md            # 产品需求(产品定义唯一权威来源)
    ├── spec/             # 技术规格,按模块编号 00~11
    ├── TODO.md           # 里程碑 M0~M9 任务清单
    └── design/           # Claude Design 导出的设计交接包(HTML 原型,只读参考)
```

文档体系自上而下:**PRD → spec → TODO**,三者通过「spec NN §M」交叉引用保持同步。

## 📄 License

MIT（见 [LICENSE](LICENSE)）
