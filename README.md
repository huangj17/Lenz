# Lenz

> 像镜头一样，一眼看懂世界。

基于 **Tauri 2** 的跨平台（macOS + Windows）截图翻译桌面工具：划词翻译、截图翻译、原图覆盖翻译。开源、本地优先、隐私友好，原生支持 LM Studio / Ollama / 任意 OpenAI 兼容接口。

## ✨ 核心功能

| 功能 | 快捷键（默认） | 说明 |
|---|---|---|
| 划词翻译（F1） | `⌥ D` / `Alt+D` | 选中文本 → 快捷键或指示图标触发，光标处弹框显示译文 |
| 截图翻译（F2） | `⌥ S` / `Alt+S` | 框选区域 → 本地 OCR → 翻译 → 弹框显示原文 + 译文 |
| 原图覆盖翻译（F3） | `⌥ F` / `Alt+F` | 框选区域 → OCR + 翻译 → 译文按原文位置覆盖在屏幕上（核心差异化） |

配套能力：菜单栏托盘常驻、翻译引擎配置（OpenAI 兼容 API / 本地 LLM）、中英双语界面、深浅色主题。

### 为什么做 Lenz

Bob 是 macOS 上广受好评的翻译工具，但仅支持 macOS、订阅制收费、对本地 LLM 不够友好。Lenz 聚焦其最核心的翻译场景，做一个**开源（MIT）、跨平台、本地优先**的替代品：

- 🖥 **跨平台**：macOS + Windows 统一体验
- 🔒 **本地优先**：OCR 走系统原生能力（macOS Vision / Windows.Media.Ocr），翻译可全程走本地 LLM，数据不出机器
- 🪶 **轻量**:Tauri 打包，目标安装包 < 15MB

## 🛠 技术栈

- **壳与后端**：Tauri 2 + Rust（截图 `xcap`、OCR 系统桥接、CGEventTap 划词监听、配置原子写）
- **前端**：React 19 + TypeScript + Vite 7 + Tailwind CSS v4 + Zustand + i18next
- **类型契约**：`tauri-specta` 从 Rust 命令导出 `src/lib/bindings.ts`，前后端共享类型

## 🚧 项目状态

开发中，按里程碑推进（详见 [`docs/TODO.md`](docs/TODO.md)）：

- ✅ **M1 项目搭建**：窗口/托盘/全局快捷键/配置体系/类型导出链路
- ✅ **M2 OCR 桥接**（macOS）：Vision 框架零权限识别打通；Windows.Media.Ocr 待 Windows 环境
- ✅ **M3 翻译引擎**：引擎抽象 + OpenAI 兼容接口
- ✅ **M4 截图翻译 F2**：选区蒙层 → 截图 → OCR → 翻译 → 弹框（macOS 已跑通）
- 🔄 **M5 划词翻译 F1**：AX 直读选中文本 + 指示图标已落地，部分交互待实机验收
- ⏳ **M6 原图覆盖 F3**、**M7 设置页交互**、**M8 完善**、**M9 打包发布**

当前以 macOS 为主开发平台，Windows 侧适配集中在后续里程碑。

## 🚀 开发

### 环境要求

- [Rust](https://www.rust-lang.org/tools/install)（stable）
- [Node.js](https://nodejs.org/) + [pnpm](https://pnpm.io/)
- Tauri 2 的[平台前置依赖](https://tauri.app/start/prerequisites/)

### 常用命令

```bash
pnpm install                                          # 安装依赖
pnpm tauri dev                                        # 启动开发（托盘 app + 主窗口）
pnpm typecheck                                        # 前端类型检查
cargo check --manifest-path src-tauri/Cargo.toml      # Rust 检查
cargo test export_bindings --manifest-path src-tauri/Cargo.toml   # 重新生成 bindings.ts
```

macOS 首次运行划词监听 / 截图相关功能时,需在系统设置中授予**辅助功能**与**屏幕录制**权限。

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

MIT（LICENSE 文件待发布时补充）
