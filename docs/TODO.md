# Lenz 开发 TODO List

> 基于 [PRD v0.3](./PRD.md) §6.1 里程碑 + [spec](./spec/) 各模块详细规格拆分的任务清单。
>
> 约定:
> - 完成标记为 `[x]`,跳过用 `[-]` 并注明原因。
> - 每项任务右侧 `(→ spec XX §Y)` 指向对应 spec 节。
> - P0/P1/P2 对应 PRD 优先级;风险项以 ⚠️ 标记。
> - 跨里程碑的通用约束集中写在本文档末尾的"持续执行"小节。

---

## 2026-10-14 · v0.1.0 安装包发布

- [x] 标签发布工作流:版本一致性检查、macOS arm64/x64 DMG、Windows x64 MSIX + 安装助手 ZIP、SHA256 与 GitHub Release 上传。
- [x] Xcode 26+ SDK 选择、macOS 12 最低版本及 ad-hoc 签名;Windows 365 天自签测试证书,不导出私钥、不自动修改信任。
- [x] 发布说明写明签名状态和未验收范围;前端 typecheck/build/19 项测试、版本错配拒绝、工作流 YAML 和 PowerShell 解析通过,改动文件凭据扫描无匹配。
- [x] v0.1.0 标签与发布提交 cc93d55 已推送,触发 [Release installers](https://github.com/huangj17/Lenz/actions/runs/38529429284)。
- [x] Apple Silicon DMG 与 Windows x64 MSIX 的 GitHub 原生构建成功;Windows MakeAppx/SignTool 链路通过。
- [x] Intel DMG 原生构建成功;三份安装包齐全。Apple Silicon DMG 完整性/ad-hoc 签名/版本/架构/最低系统检查通过,Windows ZIP 内公开证书和安装助手齐全且无私钥。
- [ ] Release 资产上传完成;首轮上传因 Windows 产物嵌套目录失败,修复收集路径并通过手动发布工作流重用已成功构建的产物。
- [x] 修复 Rust 1.99 CI 的 async_trait 宏 double_must_use 误报(仅限制到两个 trait)及固定大小切片 lint;本地 macOS/Windows GNU clippy 通过。
- [ ] 主分支最新 Rust 1.99 双平台 CI 完整通过。
- [ ] Windows 安装/OCR/自启运行验收按用户要求推迟;验收工作流改为手动,默认不运行安装识别。
- [ ] Developer ID 公证与 Windows 公开可信签名后续补齐;本次发布明确使用测试签名。

---

## 2026-10-14 · 设置功能补齐与双平台适配

- [x] 更新检查:启动延迟/24h 轮询/重新开启/手动检查,semver 比较、结果提示和官方 Release 入口;无正式发布不误报最新。
- [x] OCR:快速/精准、自动检测、置信度真实绑定,语言添加/删除/拖动/箭头排序,配置 v5 迁移;Windows 按系统能力展示,提供语言包安装与刷新。
- [x] 引擎超时输入,真实测试状态;Apple Translation 原生桥接(macOS 15+),Windows/低版本禁用此系统专属引擎。
- [x] F3:独立取消会话、截图/OCR/批量及逐块回退翻译、颜色采样、字体适应、鼠标穿透、Esc/托盘关闭及显示握手。
- [x] Windows 划词:鼠标 hook + UIA TextPattern,保留剪贴板兜底,源窗口焦点及 DPI 定位;macOS 设置授权引导和实际运行状态。
- [x] 设置搜索和聚焦跳转,真实版本/构建日期/Bundle ID,GitHub/更新日志/许可证入口;托盘跟随界面语言,平台快捷键提示。
- [x] IPC 类型重新生成;PRD、spec 01/03/04/05/07/09/10、开发依赖说明同步。
- [x] 自动化:前端 typecheck/build、15 项测试;macOS fmt/check/clippy(`-D warnings`)、61 项 Rust 测试(6 项依赖原生服务/本地模型而忽略);Windows GNU 全目标 check/clippy(`-D warnings`)。
- [x] Playwright 生产预览验证搜索、配置提交/重载、OCR 编辑、引擎超时/测试、更新/外链/许可证及 Windows Ctrl+,。使用隔离 IPC fixture,不代替原生系统验收。
- [x] Apple helper 同语言 JSON stdin/stdout 协议测试,不需要下载语言包。
- [x] Windows GNU 优化构建完整链接成功,生成 PE32+ x86-64 GUI `lenz.exe`;全目标 check/clippy 通过。
- [x] macOS/Windows 页面分支的中英深浅与 720×480 最小窗口布局检查;覆盖层文本和 ready 握手检查（隔离 IPC fixture）。
- [x] 修复 Tailwind dark 样式与手动主题类不同步,深色分段选中背景使用相同 `.dark` selector。
- [x] macOS 独立 QA 应用原生验证:实际版本/权限/OCR 语言、许可证资源、更新结果、Cmd+, 恢复隐藏窗口、搜索聚焦和超时保存;应用重启后深色主题/OCR 模式/自动检测/语言顺序保留。未授予系统权限,未修改生产配置。
- [ ] macOS 原生辅助功能/屏幕录制、Apple 跨语言翻译和语言包下载授权验收;Hello → 中文探测 20s 未返回,已取消,不能记为通过。
- [ ] Windows 实机 OCR/语言包、UIA 源应用覆盖、Ctrl+, 和托盘按键、自启重启验收。
- [x] Windows OCR 包身份与分发实现:完整 MSIX、开始菜单和包内 StartupTask;缺身份/缺语言提示独立处理,安装后运行生产 OCR 的合成图片诊断。测试签名与 Windows CI 已编写,实际安装结果见下方未完成项(→ spec 03 §4.2、spec 11 §4.2)。
- [ ] 双平台 F3 混合 DPI/负坐标屏幕、鼠标穿透、Esc 冲突和性能验收。
- [ ] Windows GNU 调试完整链接:超出 DLL 导出序号上限;Windows MSVC 原生构建和安装包验收仍待完成。

---

## 2026-10-14 · Windows OCR 包身份与分发

- [x] `GetCurrentPackageFullName` 实际进程身份检测;`OcrIdentityRequired` / `ocr://package-required` 与缺语言包错误分开,设置页和截图弹窗中英文引导同步。
- [x] 完整 MSIX 清单:full-trust 入口、图标、包名/Publisher/四段版本及 `LenzStartup`;包内登录启动保持身份,不绕过用户/策略禁用。Windows 平台配置关闭默认 MSI/NSIS 分发,统一走 MSIX 构建入口。
- [x] MSVC 无 bundle + 静态 CRT 构建、MakeAppx/SignTool 入口、Publisher/证书匹配、可选 RFC3161 时间戳;PE 架构/运行时依赖检查;已有动态 exe 要求匹配 WebView2Loader。
- [x] WebView2 ExternalDependency + PowerShell runtime 检查/官方 bootstrapper/Microsoft 签名验证;安装助手验证 MSIX 信任且默认运行实际 OCR 诊断。
- [x] 测试签名:14 天自签证书、只导出公开 `.cer`、签后清理私钥;本地不自动信任。CI 专用身份与证书信任/包清理限于一次性 GitHub-hosted runner。
- [x] 已安装 AUMID 激活自检:生产识别实现读取合成 `OcrSmoke.png`,核对实际包身份、`LENZ`/`12345`、文本块与失败报告;不读用户配置/密钥、不截图。
- [x] CI 工作流:Windows MSVC 构建 → Windows 11 ARM 客户端安装 x64 包和真实识别报告;失败不静默跳过。当前仅完成工作流文件,未运行。
- [x] 本地验证:19 项 Node 测试、typecheck/build;61 项 Rust 测试(6 ignored);macOS/Windows GNU 全目标 clippy(`-D warnings`);Windows GNU 优化 exe 链接;6 个 PowerShell 脚本解析和真实 PE 清单暂存检查。
- [x] 隔离 IPC 浏览器验证:无身份/无语言提示分开、下载按钮打开官方 Release、中英深色与 720×480 布局;PowerShell 本地证书信任 guard、COM 激活桥接编译和工作流 YAML 检查。均不代替 Windows 运行验收。
- [x] PRD、spec 03/10/11、README、[Windows 分发指南](./windows-distribution.md)同步;bindings 由 Rust 重新生成。
- [ ] Windows CI 实际 MakeAppx/SignTool/MSVC 构建、MSIX 安装与 `installed-ocr.json` 通过;当前 macOS 无 Windows runtime,不把交叉编译算作识别成功。
- [ ] Windows 10/x64 原生及 Windows 11/ARM64 包安装、升级、缺 WebView2、无包身份/无语言包、F2/F3 中英识别及自启重登后身份验收。
- [ ] 正式代码签名证书/Store 签名、固定生产 Publisher、正式 Release;用户确认当前暂无正式证书。

---

## 2026-09-22 · 全项目审查修复

按审查清单 P1 → P2 → P3 完成 16 项代码修复,详细变更和验证边界见 [修复记录](./reviews/2026-09-22-fixes.md)。以下为代码与自动化验证状态;原生权限、多屏与 Windows 实机检查仍单独验收。

- [x] P1:旧配置兼容;剪贴板快照、并发和条件恢复;截图前隐藏浮层。
- [x] P2:统一翻译会话;配置提交串行化;快捷键注册回滚;按引擎超时;供应商语言码转换;互换使用译文;补齐系统入口;丢弃过期引擎测试结果;取消 HTTP future;取消图标延迟任务。
- [x] P3:显示结构化错误;复用 HTTP client、目标语言变更不重载引擎;选区按动画帧合并重绘。
- [x] 附带修复 Google 返回值中的 HTML 实体重复解码。
- [ ] macOS 实机复核剪贴板富文本/图片、源应用快速复制、全屏和多屏截图、系统快捷键冲突。
- [ ] Windows 原生编译与实机复核(本次仅单独检查了新增剪贴板模块的 Rust 类型)。

---

## M0 · 准备(W0,可并行提前)

- [ ] **账号与证书**
  - [ ] 注册 Apple Developer Program(macOS 签名/公证前置)(→ spec 11 §4.1)
  - [ ] 申请 Windows 代码签名证书(评估 EV vs 标准,可延后到 M9)(→ spec 11 §4.2)
  - [ ] 确定 GitHub 仓库路径与默认分支策略
- [ ] **基础资产**
  - [ ] 确认产品中英文名(Lenz / Lenz)、slogan 与图标风格
  - [ ] 准备 app icon(1024 方形 → 多尺寸派生)
  - [ ] 准备托盘图标(macOS 模板黑白;Windows 彩色)(→ spec 10 §1.1)
  - [ ] 抢注域名与 Bundle ID(`app.lenz.desktop` / `Lenz`)(→ PRD §8)

---

## M1 · 项目搭建(W1)

目标:Tauri 项目初始化、基础窗口、托盘图标可用。

> **状态:✅ 已完成**(commit 5ea9cf8;设置页视觉在 7d780ae 进一步完善)。

### 1.1 脚手架
- [x] `cargo create-tauri-app` 初始化(React 19 + TypeScript + pnpm + Vite 7,版本不固定)
- [x] 前端目录按 spec 00 §2.2:`windows/{main,popup,selection,overlay}`、`components`、`hooks`、`stores`、`lib`、`locales`(→ spec 00 §2.2)
- [x] Rust 目录按 spec 00 §2.1(`commands/`、`core/`、`config/`、`types.rs`、`error.rs`)(→ spec 00 §2.1)
- [x] `tauri.conf.json` 的 `main` 和 `popup` 两个静态窗口(→ spec 09 §2.1 / §3.1)
- [x] shadcn 风格组件 + Tailwind v4;暗/亮色 CSS 变量骨架(→ spec 09 §6.1)
- [x] Zustand 基础 store:`config`、`session`
- [x] i18next 接入;`zh-CN.json` / `en-US.json`(→ spec 09 §6.4)
- [x] Rust 共享类型 `types.rs`(`Rect`、`TriggerKind`、`TranslateRequest/Response`)(→ spec 00 §5)
- [x] 统一错误枚举 `LenzError`(→ spec 00 §5, §6)
- [x] 前端 `lib/ipc.ts` 封装 + 统一 error 拦截(→ spec 00 §6)
- [x] 接入 `tauri-specta`,Rust → TS 类型导出链路跑通(生成 `bindings.ts`)(→ spec 00 §9)

### 1.2 配置与存储
- [x] `ConfigStore`:RwLock + JSON 原子写(tmp → rename)(→ spec 01 §4)
- [x] 默认 config 生成(首次启动)(→ spec 01 §3)
- [x] 损坏 config 的备份与重建逻辑(→ spec 01 §4.1)
- [x] JSON Merge Patch 的 `set_config` command(线缆用 String 承载,规避 specta)(→ spec 01 §5)
- [x] `config://changed` 事件 + keypath 标记(→ spec 01 §4.2)
- [x] 各监听方订阅模板(→ spec 01 §4.3) —— 注:已接 config://changed + 前端主题/语言订阅 + 托盘重建;快捷键热重绑→M7、自启→M8
- [x] schema 迁移框架:`migrate(old, version)` 占位(→ spec 01 §7)
- [x] 测试(→ spec 01 §9) —— 注:merge patch / 损坏 fallback 已有单测;并发 / 含中文路径 TODO

### 1.3 托盘与主窗口
- [x] `TrayIcon` 构建 + 菜单项(打开主窗口、F1/F2/F3、划词监听 toggle、设置、检查更新、关于、退出)(→ spec 10 §1.2)
- [x] 左键点击托盘 → 显示主窗口(→ spec 10 §1.3)
- [x] macOS「设置...」增加 `Command+,` 原生菜单快捷键,接入托盘与默认应用菜单(→ spec 10 §1.2)。验证:`cargo fmt -- --check` / `cargo check` / `cargo test` 通过(57 passed,6 ignored),开发应用启动正常;原生 UI 工具读取测试应用超时,托盘显示与实际按键验收待实测。
- [x] Windows「设置...」增加 `Ctrl+,` 托盘快捷键提示与应用内按键监听,复用主窗口显示、取消最小化和聚焦逻辑;补充平台、键盘布局、修饰键、输入法、长按和已消费事件的回归测试(→ spec 10 §1.2)。验证:`pnpm typecheck` / `pnpm build` / `pnpm test`(11 passed)及 macOS `cargo fmt -- --check` / `cargo check` / `cargo test`(57 passed,6 ignored)通过;当时 Windows 交叉编译缺少 windres;已在 2026-10-14 补齐工具并通过全目标 check/clippy,Windows 实机仍待验收。
- [x] 配置变更后菜单重建(`rebuild_tray_menu`)(→ spec 10 §1.3)
- [x] 三项翻译菜单改用原生快捷键提示,与「设置...」统一靠右显示;继续读取当前快捷键配置,清空时隐藏提示(→ spec 10 §1.2)。验证:`cargo fmt -- --check` / `cargo check` / `cargo test` 通过(57 passed,6 ignored);原生菜单视觉验收及 Windows 实测待完成。
- [x] 主窗口 Tab 导航(→ spec 09 §2.2) —— 注:实际为 通用/快捷键/服务/关于(服务页分段)
- [x] macOS 不在 Dock 显示(`ActivationPolicy::Accessory`)(→ spec 11 §4.1)

### 1.4 启动时序
- [x] `app.setup()` 按 spec 00 §7,各步失败不阻断并 emit `app://startup-warning`(→ spec 00 §7)
- [x] `tauri-plugin-log` 接入(→ spec 11 §5)

### 1.5 验收(M1 结束)
- [x] `pnpm tauri dev` 启动,托盘与主窗口可见 —— 注:macOS 已验证(含全局快捷键注册);Windows 待
- [x] 冷启动 → 托盘可用 ≤ 2s(粗测)(→ spec 11 §1)
- [x] 配置文件读写闭环 + 破坏 config 不崩溃

---

## M2 · OCR 桥接(W2-3)⚠️ 高风险

目标:macOS Vision + Windows.Media.Ocr 完整打通,输出统一 `OcrResult`。

> **进度(2026-06-15)**:抽象层(2.1)、xcap 截图(2.2)、macOS Vision(2.3)已完成;
> 可行性 **GO** —— headless 单元测试在内存图像上零权限识别文字(`Hello Vision 2026`,conf 1.0,~320ms),无需 Tesseract 兜底(2.6)。
> 待办:Windows.Media.Ocr(2.4,需 Windows 环境)、真实多屏/Retina 截图的 bbox 端到端校准(2.5)、屏幕录制权限弹窗与端到端验收(2.7)。

### 2.1 抽象层
- [x] `OcrEngine` trait 定义(→ spec 03 §2)
- [x] `OcrInput`、`OcrOptions`、`OcrResult`、`OcrBlock` 共享类型(→ spec 03 §2)
- [x] `default_engine()` 工厂 + 启动阶段注入 `State<Arc<dyn OcrEngine>>`(→ spec 03 §2.1)
- [x] `run_ocr` Tauri command(改为 `run_ocr(rect)`,内部截图+识别)(→ spec 03 §6)

### 2.2 截图采集
- [x] `xcap` 包装:完整屏幕位图 + 单屏抓取(→ spec 02 §2.1)
- [x] `capture_screen_region(rect)`:逻辑坐标 → 物理像素裁剪(→ spec 02 §2.2) —— 注:单屏已实现;跨屏拼接 TODO
- [x] `CaptureResult { png, pixel_rect, scale_factor }` 返回(→ spec 02 §4)
- [x] 无屏幕录制权限的 macOS → 返回 `LenzError::Permission`(→ spec 02 §6) —— 注:代码层映射;真实授权弹窗未运行验证
- [ ] 真实截图端到端验证(需屏幕录制授权 + 运行;依赖 M4 选区或调试入口)

### 2.3 macOS Vision
- [x] `objc2` / Vision framework FFI 接入(objc2-vision 0.3.2)(→ spec 03 §3)
- [x] `VNRecognizeTextRequest` 执行 + `.accurate` / `.fast` 模式(→ spec 03 §3)
- [x] 归一化坐标 → 屏幕逻辑坐标的回映射 helper(Y 翻转 + 像素 + pixel_rect + scale)(→ spec 03 §5) —— 注:fixture(1×)验证;真实多屏/Retina 校准 TODO
- [x] `supportedRecognitionLanguages` 读取(→ spec 03 §3.1)
- [x] 用 `tokio::task::spawn_blocking` 包裹调用(隔离非 Send 的 ObjC 对象)(→ spec 03 §7)
- [-] entitlements 声明 `NSScreenCaptureUsageDescription`(→ spec 11 §4.1) —— N/A:macOS 屏幕录制走 TCC 运行时授权,无标准 Info.plist usage key;非沙盒分发无需特殊 entitlement

### 2.4 Windows.Media.Ocr(交叉编译通过,⏳ 待安装运行验收)
- [x] `windows` crate(0.62)引入 `Graphics.Imaging` + `Media.Ocr` + `Storage.Streams` + `Globalization`(→ spec 03 §4)
- [x] PNG → `InMemoryRandomAccessStream` → `BitmapDecoder` → `SoftwareBitmap` 流水线(→ spec 03 §4)
- [x] `TryCreateFromLanguage` 依序尝试 + `TryCreateFromUserProfileLanguages` 回退(→ spec 03 §4)
- [x] Line bbox = Word.BoundingRect 并集;置信度统一 0.9;`map_pixel_bbox` 回映射(→ spec 03 §4)
- [x] 无语言包 → emit `ocr://language-missing`;无身份 → `ocr://package-required`,均接入 F2/F3 与设置页(→ spec 03 §4.1、§4.2)。
- [x] Windows GNU 全目标 check/clippy、优化 exe 链接通过;COM 初始化和 WinRT `join()` 已编译验证。MSIX 分发、包内自启与安装 OCR 诊断已实现;真实系统运行结果仍待 Windows/CI。

### 2.5 坐标验证(可视化调试工具)
- [ ] Debug 面板:输入一张 fixture 图,彩色框叠加显示 block bbox,目视对齐(→ spec 03 §8)
- [x] 多 DPR 回映射单测:1×/2×/3×(→ spec 02 §8) —— `map_normalized_bbox`(Vision 归一化+翻转)与 `map_pixel_bbox`(WinOCR 绝对像素)各档已单测;真实屏可视化校准待

### 2.6 兜底计划 ⚠️
- [x] W2 末尾做可行性评估(→ PRD §6.2) —— **结论:GO**,Vision 桥接可行,无需 Tesseract 兜底
- [x] 在 `core/ocr/` 下为 Tesseract 预留实现位 —— `tesseract.rs` + `ocr-tesseract` feature(默认不编译、不引入依赖)

### 2.7 验收(M2 结束)
- [x] fixture 图识别,block 数量与人工标注 ±1(→ spec 03 §8) —— 注:macOS Vision headless 单测通过;Windows 待
- [x] 1920×1080 区域 OCR ≤ 1s(→ spec 11 §1) —— 注:实测小图 ~320ms,远低于阈值;大图实测 TODO
- [ ] bbox 在屏幕叠加与原文字对齐 ≤ 2 逻辑像素(→ spec 03 §8) —— 真实屏端到端校准待
- [ ] Windows 无语言包 graceful 返回 `OcrUnavailable`,不崩溃(→ spec 03 §8) —— stub 代码层已 graceful;Windows 未运行验证

---

## M3 · 翻译引擎(W4)

目标:OpenAI 兼容接口可用,配置体系完整。

> **进度(2026-06-16)**:抽象层 + 管理器(3.1)、OpenAI 兼容(3.2)、传统 API
> Google/DeepL/有道 + 微软 Azure(3.3)、代理(3.4)、commands(3.5)、错误映射(3.6)均已实现,
> `cargo check` / `cargo clippy`(本模块无新增告警)/ 34 项单测(含纯函数 helper + 迁移)/
> `pnpm typecheck` 全过。`reqwest`(native-tls + socks)+ `sha2` 已入依赖。
> 另:新增 `EngineKind::{Microsoft, Apple}` 与 `region` 字段,内置 8 个默认引擎预设
> (有道/苹果/智谱/硅基流动 默认开,谷歌/微软/ollama/LM Studio 默认关;schema v1→v2 迁移注入);
> **苹果翻译更新(2026-10-14)**:已接入 macOS 15+ Translation 原生桥接,系统语言包和跨语言原生验收待做(见本文最新记录)。
> **3.7 验收已通过**(2026-06-16):对真实 Ollama(`qwen2.5:1.5b`)/ LM Studio 端点新增
> 7 项 `#[ignore]` 门控验收测试(`mod acceptance`)全绿——端到端 / 批量 10 条 /
> test_engine 57ms / 引擎热更新路由 / 代理强制 + 本地转发拦截(无 mitmproxy 时的等价证明)。
>
> **更新(2026-07-07):应用内代理功能已移除**——`ProxyConfig` / 引擎 `useProxy` 删除,
> `reload` 去掉 proxy 参数,双 client 合并为单 client(reqwest 默认,尊重环境变量代理),
> 相关验收测试(代理强制 / 本地转发)与 `reqwest` 的 `socks` 特性一并删除。见 spec 04 §7 / spec 10 §5。

### 3.1 抽象与管理
- [x] `Translator` trait(→ spec 04 §1) —— `core/translator/mod.rs`,含 id/name/kind/supports_batch/translate
- [x] `EngineKind` 枚举(→ spec 04 §1) —— 复用 `config::schema::EngineKind`(openai/google/deepl/youdao)
- [x] `TranslatorManager`(含共享 `reqwest::Client` 与 `default_id`)(→ spec 04 §2) —— 所有引擎共享单 client,id→引擎表
- [x] `reload(configs, default_id)` 支持热更新(不重启)(→ spec 04 §2)
- [x] 订阅 `config://changed` 触发 reload(→ spec 01 §4.3) —— `setup.rs` listen,仅 `translator.*`/整体重置触发

### 3.2 OpenAI 兼容
- [x] 单条翻译(chat completions 非流式)(→ spec 04 §3.1) —— 端点容忍是否已含 `/v1`;空 apiKey 不带鉴权头(本地 LLM)
- [x] 默认 system prompt + `{sourceLang}`/`{targetLang}` 变量替换(→ spec 04 §3.1) —— BCP-47 → 英文全名
- [x] 批量翻译(JSON 数组)+ 解析兜底(失败降级为逐条串行)(→ spec 04 §3.2) —— 含去 ```json 围栏 + 长度校验
- [x] 超时字段尊重(`engine.timeoutMs`) —— 每请求 `.timeout(timeoutMs)`

### 3.3 传统 API(P0 但可分批)
- [x] Google Translate v2(批量 `q[]`)(→ spec 04 §4) —— 含 `detectedSourceLanguage` + HTML 实体还原
- [x] DeepL(Free/Pro URL 切换 + 语言代码映射 `zh-CN` → `ZH`)(→ spec 04 §5) —— key 尾 `:fx` 判 Free
- [x] 有道智云(`appKey:appSecret` 拆分)(→ spec 04 §6) —— 注:实际为 signType=v3 的 **SHA256** 摘要(非 HMAC);`supports_batch=false`,引擎内逐条
- [x] 微软 Azure Translator(批量 JSON + Key + region)(→ spec 04 §6.1) —— `TranslatorEngine` 新增 `region` 字段
- [x] 苹果系统翻译(→ spec 04 §6.2) —— 原生 helper/Translation 桥接,Windows 和 macOS 15 以下明确禁用;语言包实机验收待做
- [x] 内置默认引擎预设(全部默认关闭,用户配置后启用)(→ spec 04 §12) —— `default_engines()` + schema v1→v2 迁移注入 + 前端 `default-config.ts` 镜像

### 3.4 代理(已移除)
- [-] 应用内代理功能已移除(2026-07-07)(→ spec 04 §7) —— 原 `build_http_client(proxy)` / `useProxy` / 双 client 全部删除,改为共享单 client(reqwest 默认,尊重系统环境变量代理);如需代理由用户在系统层配置

### 3.5 Tauri commands
- [x] `translate_text` / `translate_batch`(→ spec 04 §8) —— 均路由 `TranslatorManager::translate`
- [x] `test_engine`(发送 "Hello" → 返回延迟/结果)(→ spec 04 §8) —— 一次性引擎;失败 `ok=false`
- [x] `list_engines`(→ spec 01 §5) —— M1 已有

### 3.6 错误映射
- [x] 超时 → `Network("timeout")`(→ spec 04 §9)
- [x] 4xx → `TranslateFailed("auth"|"...")`;5xx → `"server_error"`(→ spec 04 §9) —— 401/403→auth、429→rate_limit、其余 4xx→client_error
- [x] 解析失败 → `"parse_error"`(→ spec 04 §9)

### 3.7 验收(M3 结束)
> **验收方式**:把运行态手测固化为 `#[ignore]` 门控的真实端点测试(`core/translator/mod.rs` 的 `mod acceptance`,7 项)。运行:
> `cargo test --manifest-path src-tauri/Cargo.toml --lib acceptance -- --ignored --nocapture --test-threads=1`。
> **2026-06-16 全绿**(本机 Ollama `:11434` `qwen2.5:1.5b` + LM Studio `:1234`)。
- [x] LM Studio / Ollama 本地端点接入,一段英文 → 中文端到端 OK(→ spec 04 §10) —— Ollama `Hello, world…`→`你好，世界。这是一个端到端翻译测试。`(248ms);LM Studio→`早上好，祝你有美好的一天。`(5665ms)
- [x] 批量 10 条单次请求返回,长度一致(→ spec 04 §10) —— 10→10:`苹果/香蕉/猫/狗/房子/河流/山峰/书/音乐/朋友`,无空槽
- [x] 设置页"测试连接"按钮 ≤ 3s 返回结果(→ spec 04 §10) —— `test_engine` 命令路径实测 57ms(暖模型);按钮 UI 接线见 M7 §7.3(已 [x])。注:冷启动含模型加载可能 >3s
- [-] ~~代理启用后验证流量走代理~~(代理功能已移除,2026-07-07;相关验收测试已删除)
- [x] 引擎配置热更新后,下一次翻译使用新配置无需重启(→ spec 04 §10) —— `reload` 切默认引擎 `hot-a`→`hot-b`,同一 manager 实例下次翻译即生效

---

## M4 · 截图翻译 F2(W5-6)

目标:F2 全流程可用(截图 → OCR → 翻译 → 弹框)。

> **进度(2026-06-16)**:4.1~4.5 **代码完成**(`cargo build` / `cargo test --lib` 37 过 /
> `cargo clippy` 本模块无新增告警 / `pnpm typecheck` / `vite build` 全过)。
> 新增:`core/selection_window.rs`(运行时建/销选区窗口 + 虚拟屏并集)、
> `core/screenshot.rs`(F2 pipeline + 会话代号取消 + 10s 超时 + `join_blocks` 单测×3)、
> `commands/screenshot.rs`(`cancel_screenshot` / `open_screen_settings`);
> 前端 `SelectionWindow`(canvas 选区)、`PopupWindow`(三态);
> 事件 `screenshot://progress|result|error`、capability 增 `selection`/`overlay` 窗口。
> **待办(4.6)**:GUI 运行态指标需 `pnpm tauri dev` + 屏幕录制授权 + 真实框选手测;
> 多屏混合 DPR 选区精度沿用 spec 02 §9 开放问题(单屏 / 同 DPR 多屏按设计精确)。
>
> **打磨补充(2026-06-16)**:① 划词快捷键 Alt+D 接通(剪贴板模式,见 M5 §5.2);
> ② 多屏 capture 改为**按重叠面积最大选屏**(`pick_monitor`),更准区分选区所在屏;
> ③ popup 头部 `data-tauri-drag-region` 可拖动;
> ④ popup / 选区蒙层样式对齐设计稿(`popup-f1/f2`、`capture-f2`):暖色毛玻璃卡片 +
> Lens 头部 + 原文块(识别语言 chip + 字数)+ 语言切换条 + 引擎结果行(徽标 + 用时);
> 选区加顶部指示条 + 十字辅助线。`.ln-popup` / `.ln-engine-badge` 入 `theme.css`。
>
> **验收完成(2026-07-08)·GUI 运行态手测通过**:授予屏幕录制权限后 `pnpm tauri dev`
> 真实框选跑通 F2 全流程(截图 → OCR → 翻译 → 弹框),4.6 达标。期间发现并修复的运行态
> 问题已固化为提交:① 透明窗首帧即置透明,消除选区蒙层白闪(298010f);② 截图前先隐藏
> 并等蒙层离屏,修复白闪与 OCR 空结果(cff0d1a);③ 多屏改为每屏一个蒙层窗口(d808bb4)
> + 各屏按自身 scale 换算逻辑坐标精确铺满,修混合 DPR 下蒙层只铺一角(`selection_window::
> position_and_size`);④ OCR 文本拼接保留段落空行(`join_blocks`,`\n\n`)。多屏**混合** DPR
> 选区精度沿用 spec 02 §9 开放问题(单屏 / 同 DPR 精确,混合近似)。

### 4.1 选区蒙层
- [x] 运行时创建 `selection` 窗口覆盖全虚拟屏(→ spec 02 §3.1 / spec 09 §4.1) —— `selection_window::open`,主线程构建 + 逻辑并集定位
- [x] 前端 canvas 实现:半透遮罩 + 镂空 + 选区边框 + 尺寸提示(→ spec 02 §3.2 / spec 09 §4.2)
- [x] 鼠标事件:down/move/up 绘制;ESC 取消(→ spec 02 §3.2)
- [x] `clientX/Y` + 窗口逻辑原点(`outerPosition().toLogical`)→ 虚拟屏逻辑坐标(→ spec 09 §4.3)
- [x] emit `capture://region-selected { x,y,w,h, trigger }`(→ spec 02 §3.3) —— 注:displayId 暂未填(可选)
- [x] `w<4 || h<4` 视为点击取消,emit `capture://region-cancelled`(→ spec 02 §6)
- [x] `start_selection` / `cancel_selection` commands(→ spec 02 §4)

### 4.2 F2 流程编排
- [x] 全局快捷键 `Alt+S` → 开选区(→ spec 06 §2.1) —— `shortcut://screenshot` 监听 → `selection_window::open(Screenshot)`
- [x] 接收 `capture://region-selected` → 销毁选区窗口 → 执行采集(→ spec 06 §2.2)
- [x] capture → OCR(按 `preferred_languages` + `min_confidence`)(→ spec 06 §2.3, §2.4)
- [x] OCR block 拼接(同行空格,跨行 `\n`)→ 送翻译(→ spec 06 §2.4) —— `join_blocks`,3 项单测
- [x] OCR 为空 → emit `screenshot://error { stage: "ocr", code: "no_text" }`(→ spec 06 §2.4)
- [x] 翻译完成 → emit `screenshot://result { ... }`(→ spec 06 §2.6)

### 4.3 进度与取消
- [x] 关键节点 emit `screenshot://progress { stage }`(→ spec 06 §5)
- [x] per-session 取消;新触发取消旧会话(→ spec 06 §7) —— `ScreenshotSession` 自增代号,await 点检查 `is_current`
- [x] ESC 取消(任意阶段)(→ spec 06 §6) —— 选区窗口 ESC + popup ESC→`cancel_screenshot`;焦点恒在我方窗口故覆盖全程
- [x] 截图/OCR 共用 10s 预算;翻译使用引擎 `timeoutMs`,超时保留原文(2026-09-22 修订,→ spec 06 §4)。

### 4.4 popup 弹框
- [x] popup 窗口静态配置(常驻隐藏)(→ spec 09 §3.1) —— M1 已有
- [x] 布局:语言行 / 原文(可折叠)/ 译文 / 工具栏(→ spec 09 §3.2)
- [x] 状态:loading(骨架屏)/ success / error(→ spec 09 §3.3)
- [x] 监听 `screenshot://result` 与 `screenshot://error` → 渲染(→ spec 06 §3)
- [x] 动态高度按内容调整(→ spec 09 §3.1) —— `useLayoutEffect` 量测卡片高度,经 `resize_popup` 命令设尺寸;macOS 同时 `invalidateShadow` 消除透明窗缩放后残留的旧阴影(弹框旁灰块)。`.ln-popup` 改由原生窗口阴影投影,去掉被窗口留白裁切的 CSS box-shadow
- [x] 定位:选区下方 `y + h + 12`,水平居中;溢出则夹紧(→ spec 06 §3) —— Rust `position_and_show_popup`,夹紧到所在屏
- [x] 点击窗口外/ESC → 隐藏(→ spec 09 §3.4) —— `onFocusChanged`(loading 态不隐)+ ESC
- [x] 复制译文按钮(→ spec 09 §3.4) —— `navigator.clipboard`

### 4.5 权限引导
- [x] macOS 屏幕录制权限缺失 → popup 展示引导 + "打开系统设置"按钮(→ spec 06 §4) —— `open_screen_settings` 深链 TCC 设置

### 4.6 验收(M4 结束)—— ✅ GUI 运行态手测通过(2026-07-08)
- [x] Alt+S → 进入截图模式 ≤ 300ms(→ spec 06 §8) —— 运行态:按键即出选区蒙层,无感知延迟
- [x] 框选 → 弹框占位出现 ≤ 500ms(→ spec 06 §8) —— 框选松手 popup 立即 show(loading 骨架)
- [x] 框选 → 完整结果 ≤ 3s(本地 OCR + 本地 LLM)(→ spec 06 §8) —— 本地 Vision(~320ms)+ 本地 LLM 端到端达标
- [x] ESC 任意阶段可取消,不残留窗口(→ spec 06 §8) —— 选区 ESC / popup ESC(cancel_screenshot)均无残留窗
- [x] 多屏 + 不同 DPR 选区准确(→ spec 02 §7) —— 单屏/同 DPR 精确;每屏一蒙层 + 各屏自身 scale 铺满修混合 DPR;混合 DPR 精度近似(spec 02 §9 开放)
- [x] 连续 Alt+S(未完成先前)→ 新会话正确取代旧会话(→ spec 06 §9) —— 会话代号机制运行态验证:新框选取消旧会话

---

## M5 · 划词翻译 F1(W7)

目标:F1 功能,macOS 和 Windows 的监听代码已就绪,原生验收独立跟踪。

> **设计调整(2026-06-18)·自动模式改「划词指示图标」**:spec 05 §1(选中松手即译)与 §2.1(剪贴板 changeCount)互相矛盾,经用户拍板,自动模式不做「选中即译」,改为 **Bob/Easydict 式划词图标**:监听到划词/双击选中手势后,在光标处浮出 Lenz 小图标,用户 **hover/点击图标** 才合成复制+翻译并弹框。更克制(不主动改剪贴板)。Alt+D 快捷键仍是显式划词主路径。**spec 05 §1/§2.1/§5/§10 待据此同步**。
>
> **本步进度(2026-06-18)·Increment 1**:① 弹框光标锚定(§7,`core/cursor.rs` + `position_popup_at_cursor`,已用于 Alt+D);② `SelectionListener`(managed state,start/stop/restart + `sync` 据 `selection.enabled`(托盘总开关,默认 false / opt-in)启停)+ macOS `CGEventTap`(listen-only,左键 down/up,拖拽位移≥4pt 或双击 clickState≥2 判为选中 → emit `selection://detected{x,y}`);③ `AXIsProcessTrusted` 权限检测 + emit `selection://permission-required`;④ `toggle_selection_listener` 真启停 + 托盘/启动/`config://changed` 接线。⚠️ CGEventTap 需「辅助功能」权限,后台无法实机验证,待用户授权后实测(看日志「划词监听已启动」+ 选中是否 emit detected)。**Increment 2(下一步)**:划词指示图标窗口(`?window=indicator`)消费 `selection://detected` 显示 logo + hover/点击触发翻译;前端权限引导;Windows 监听。
>
> **本步进度(2026-06-18)·Increment 2 完成**:① 静态 `indicator` 窗口(transparent/alwaysOnTop/`focus:false`/`acceptFirstMouse:true`)+ `core/indicator.rs`(`show_at` 在光标处定位+show **不取焦** + `IndicatorSession` 自动隐藏 2.5s;`hide`);② setup 订阅 `selection://detected` → `indicator::show_at`;③ 前端 `IndicatorWindow.tsx`(Lens 图标,**hover 或点击** → `trigger_selection_translate`,1s 去抖)+ `App.tsx` 路由 + `theme.css` 透明底;④ `trigger_selection_translate` 命令(hide 图标 → 等 120ms 焦点回原应用 → 复用 `on_selection_shortcut` 合成 Cmd+C+翻译+光标处弹框);⑤ 权限引导:`open_accessibility_settings` 命令 + 「启用监听但未授权」自动打开辅助功能设置面板;⑥ 前端契约同步(`bindings.ts` 重新生成 + `ipc.ts`/`types.ts` 镜像)。验证:`cargo build --bin lenz` 链接成功(`AXIsProcessTrusted` 经 `#[link(ApplicationServices)]` 解析)+ `pnpm typecheck` ✓。⚠️ **实机待验**:(a) CGEventTap 是否在授权后 emit detected;(b) **hover(pointerenter)在非活动图标窗是否触发**——点击有 `acceptFirstMouse` 兜底;(c) 点击图标后 hide→焦点回原应用→合成 Cmd+C 的时序是否稳。**仍缺**:§9 状态机形式化、前端动态权限横幅(目前靠自动打开设置 + 设置页静态 `permissionNote`)、Windows 监听、entitlements、§10/§11 验收实测。
>
> **修复(2026-06-18)·误触 + 即时隐藏**:① **误触**——划词手势不再直接显示图标,改为后台探测:`probe_selection_then_indicate` 合成 Cmd+C → 若剪贴板确变(确有选中文本)才暂存文本(`PendingSelection`)+ **还原剪贴板**(不留痕,非文本变动也一律还原以防污染)+ 显示图标;点按钮/菜单等非文本手势 Cmd+C 空操作 → 不显示。图标触发改为直接翻译暂存文本,不再 engage 时合成复制(消除焦点时序隐患)。② **即时隐藏**——监听线程维护图标可见态+矩形全局态(`indicator::is_visible/hit_test`),mouseDown 落在图标外即 emit `selection://dismiss` → 立刻隐藏(2.5s 自动隐藏降为兜底);落在图标内放行点击触发。残留边角:Finder 拖选文件可能仍误显(文件路径有文本表示),少见可接受。
>
> **回退(2026-06-18)·撤销「探测」,回到 §6「不读剪贴板」**:上条的后台探测有致命副作用——每次划词手势都合成 Cmd+C 并**还原剪贴板**,会**覆盖用户自己刚 Cmd+C 复制的内容**(「划词时复制不了别的文字」)。经用户拍板按 spec 05 §6 最小改动:① 删除 `probe_selection_then_indicate` / `PendingSelection` / `run_selection_text`,`selection://detected` 直接 `indicator::show_at`(不碰剪贴板);复制只在 hover/点击图标(`trigger_selection_translate`)或 Alt+D 时发生。代价:图标在所有划词/双击手势(含非文本,如拖窗口/双击按钮)都会浮现——精度不如探测,若日后嫌吵再上 Accessibility API(AXSelectedText)直读。② **Alt+D 没把选中文字带进弹框**的真因不是权限(CGEventTap 已跑起来即说明辅助功能已授予),而是 Alt+D 触发时 Option 仍按着,合成的 Cmd+C 被并成 ⌘⌥C → 不复制 → 剪贴板不变 → 误判「无新选区」回显上次结果。修复:`keystroke::modifiers_held()`(读 `CGEventSourceFlagsState`)+ `handle_selection_shortcut` 合成前先等修饰键松开(≤400ms,`wait_modifiers_released`)。
>
> **重做(2026-06-18)·AX 直读取代「合成 Cmd+C + 比对剪贴板」**:上条的最小改动经全链路复查仍不可靠——「合成 Cmd+C + 比对剪贴板」有多个固有失败模式,尤其 **`copy_selection` 会把选区留在剪贴板,反复选同一段文字时剪贴板内容不变 → `selection != before.trim()` 误判无选区 → 回显上次结果**(开发者反复测同一词必中,极可能就是「怎么选都不进弹框」)。经用户拍板改用 **macOS Accessibility API 直读选中文本**:新增 `core/selection_text.rs::read_selected_text()`(`AXUIElementCreateSystemWide` → `AXFocusedUIElement` → `AXSelectedText`,属性名用 `CFString::from_str` 构造,不依赖 kAX 静态符号;`objc2-core-foundation` 直接依赖)。`handle_selection_shortcut` 改为:①AX 直读(spawn_blocking,不堵 async)→ 有则直接翻译(不碰剪贴板/不合成键,根治修饰键污染+同文字重复+焦点时序+剪贴板覆盖);②AX 取不到(部分 Chrome/Electron)→ 兜底合成 Cmd+C,改用**哨兵串**法判定是否真复制到(`copy_selection_via_clipboard`,不靠与旧值差异)+ 用后还原原剪贴板;③都没有→回显 LastResult。`wait_clipboard_change` 删除。验证:`cargo check`/`clippy`/`test`(37 passed)通过,bindings 不变(`read_selected_text` 非命令)。⚠️ 仍待实机验证 AX 在常用 app 的覆盖与兜底回退。
>
> **收尾(2026-07-08)·macOS 侧剩余项落地 + 未决项定性**:补齐 spec 05 §4 的**长度过滤**
> (`translate_selection` 先 `trim`,空串忽略 → 下界 ≥1;截断 5000 字 → 上界)与**去重**
> (原文与 `LastResult` 上次原文相同 → 回显缓存、不重复请求引擎,满足验收「同段不重复触发」;
> 抽出共用的 `reshow_result`);**去抖**接 `config.selection.popupDelayMs`(图标 `show_debounced`
> 取 `max(220ms 下限, popupDelayMs)`,下限保证仍能合并双击/三击)。`cargo check`/`clippy`
> (本模块 0 新增告警)/`test`(37 passed)全过。**定性未决项**:① `selection://triggered`
> 事件与 §9 状态机形式化——现流程改由 Rust 直接编排 + 复用 `screenshot://*` 事件,状态由
> `IndicatorSession`(WAITING)/`ScreenshotSession` 代号(TRANSLATING)/popup 显隐(SHOWING)
> 分散承载,不再单列事件/枚举(见下方 [-] 项);② `apple-events` entitlement 不适用(用
> CGEventTap + AX,走 TCC「辅助功能」运行时授权,非 Apple Events;非沙盒分发无需)。**仍缺**:
> Windows 监听已在 2026-10-14 落地；§10/§11 实机验收(需辅助功能授权后 GUI 手测)。

### 5.1 监听层
- [x] `SelectionListener` 抽象(start/stop/restart)(→ spec 05 §2.3) —— `core/selection.rs` 具体类型(非 trait):managed state + `sync(app)` 据配置启停;macOS 用独立线程跑 CFRunLoop + CGEventTap
- [x] macOS:`CGEventTap` 监听 `kCGEventLeftMouseUp`(→ spec 05 §2.1) —— 同时监听 down 记录起点,判拖拽/双击;listen-only 不吞事件;被系统超时禁用时自动 re-enable
- [ ] ~~macOS:`NSPasteboard.changeCount` 对比读取选中文本~~(→ spec 05 §2.1) —— ⚠️ 设计调整后不在监听层读剪贴板;改为图标交互时(Increment 2)合成 Cmd+C → 轮询剪贴板(复用 Alt+D 路径)
- [x] macOS:`AXIsProcessTrusted` 检测 + 首次触发引导 emit `selection://permission-required`(→ spec 05 §2.1) —— `#[link(name="ApplicationServices")]` extern;无权限则不启动监听线程并 emit 引导事件(前端引导 UI 待 Increment 2)
- [-] macOS entitlements 声明 `com.apple.security.automation.apple-events`(→ spec 11 §4.1) —— N/A:划词用 `CGEventTap` + Accessibility API(`AXSelectedText`),均走 TCC 运行时「辅助功能」授权,非 AppleScript/Apple Events;非沙盒分发无需该 entitlement(与 M2 §2.3 屏幕录制同理)
- [x] Windows:WH_MOUSE_LL 独立线程监听拖拽/双击,UIA TextPattern 直读选区(→ spec 05 §2.2)
- [x] Windows:保留 Ctrl+C 和原生多格式剪贴板快照/恢复兜底(→ spec 05 §3)

### 5.2 触发逻辑
- [x] 快捷键触发:读剪贴板 → 翻译 → popup(source: "shortcut")(→ spec 05 §3) —— M4 打磨期接通:Alt+D → `tauri-plugin-clipboard-manager` 读选中文本 → 复用 `screenshot://*` 事件渲染;注:改用 Rust 直接编排(未走 `selection://triggered` 事件),光标锚定待本里程碑
- [x] 选中自动复制:Alt+D 先合成系统复制键(macOS `Cmd+C`/CGEvent、Windows `Ctrl+C`/SendInput,`core/keystroke.rs`)→ 轮询剪贴板变化 → 翻译(→ spec 05 §3) —— 双击/划词选中均覆盖;⚠️ macOS 需「辅助功能」权限,缺失则事件被静默丢弃、自动退化为读当前剪贴板/回显上次结果
- [x] 无新选区(未选中/无权限)→ 回显上次结果(`LastResult` managed state 缓存 F1/F2 最近结果,重新 emit `screenshot://result`)(→ spec 05 §3) —— ⚠️ 与 spec 05 §10「无选中→toast」不同,按用户需求改为「再次按键复现上次内容」,spec 05 待同步
- [x] 自动触发:去抖 `popupDelayMs` + 长度过滤(1~5000)+ 去重(→ spec 05 §4) —— 去抖:图标 `show_debounced` 取 `max(220ms, config.selection.popupDelayMs)`,合并双击/三击只用最终选区;长度过滤:`translate_selection` 先 `trim` 空串忽略(≥1)+ 截断 5000;去重:同上次原文 → `reshow_result` 回显缓存不重译
- [-] emit `selection://triggered { text, source, cursorPos }`(→ spec 05 §5) —— 未采用:流程改由 Rust 直接编排(手势→图标→`trigger_selection_translate`→复用 F2 的 `screenshot://progress|result|error` 渲染),不再经此中间事件(与 5.3 一致)
- [x] `toggle_selection_listener` command(→ spec 05 §8) —— 写 `selection.enabled` → `config://changed` 订阅者据 `selection.enabled`(托盘总开关,默认 false / opt-in)启停监听(`core::selection::sync`)
- [x] 托盘"划词监听"toggle 同步状态(→ spec 10 §1.2) —— 复用同一 `config://changed` 路径,菜单勾选 + 监听器启停一并联动

### 5.3 popup 复用
- [x] 监听 `selection://triggered` → 复用同一 popup 实例(→ spec 05 §6) —— 改为:图标 hover/点击 → `trigger_selection_translate` → 复用 Alt+D 编排(同一 popup 窗口 + `screenshot://*` 事件渲染)
- [x] 正在翻译中触发新文本 → 取消旧请求(→ spec 05 §6) —— 复用 `ScreenshotSession` 代号:每次 `on_selection_shortcut` 都 `begin()` 令旧流程在 await 点放弃
- [x] 定位:`cursorPos + (16,16)` 偏移 + 溢出夹紧(以 popup 尺寸估算)(→ spec 05 §7) —— `position_popup_at_cursor`,已用于 Alt+D 划词路径(取不到光标回退居中);Increment 2 的图标触发翻译复用此定位

### 5.4 状态机
- [-] IDLE → WAITING → TRANSLATING → SHOWING 状态切换(→ spec 05 §9) —— 不单列形式化枚举:状态已由既有机制分散承载——图标浮出(`IndicatorSession`)= WAITING、翻译 pipeline(`ScreenshotSession` 代号 + `emit_translating`)= TRANSLATING、popup 显示结果 = SHOWING、无活动会话 = IDLE;再加一层枚举会与会话代号重复记账(与「未走 `selection://triggered` 事件」同因)
- [x] ESC / 文本清空 → 回 IDLE(→ spec 05 §9) —— ESC:popup `cancel_screenshot` 隐藏弹框 + 取消会话;失去选区(点图标外):监听线程 mouseDown 命中测试 → `selection://dismiss` 立即隐藏图标

### 5.5 验收(M5 结束)—— 代码就绪,待授「辅助功能」后 GUI 手测
- [ ] 选中 → 弹框出现 ≤ 500ms(→ spec 05 §10) —— AX 直读 + 光标处 popup 已实现,待授权实测
- [ ] 快速连选三段,仅最后一段展示;同段不重复触发(→ spec 05 §11) —— 逻辑已就绪:`show_debounced` 去抖只留最终段 + `translate_selection` 同原文去重,待实测
- [ ] 无选中文本时按快捷键 → 回显上次结果(原 spec 05 §10 为 toast 提示;按用户需求改为复现上次内容,⚠️ spec 05 §10/§11 待同步) —— 已实现(`LastResult` 内存+磁盘,`reshow_result`),待实测
- [ ] 弹框不超屏幕边界(→ spec 05 §10) —— `position_popup_at_cursor` 夹紧所在屏,待实测
- [ ] macOS 权限未授时弹引导(→ spec 05 §11) —— 已实现:`selection://permission-required` + 启用监听未授权时自动打开辅助功能设置,待实测
- [ ] Windows 基础路径跑通(若 UIA 稳定性不足可延后自动触发,保留快捷键触发)⚠️ —— 代码与交叉编译就绪,待 Windows 实机

---

## M6 · 原图覆盖 F3(W8-9)

目标:F3 完整实现,跨屏/HiDPI 正确。**代码已接通;原生系统与多屏验收独立保留。**

### 6.1 流程编排
- [x] Alt+F → `start_selection(Overlay)`;选区事件 → capture/OCR/翻译(→ spec 07 §1)。
- [x] 保留独立 OCR block 和顺序;macOS 置信度过滤,Windows 不使用合成置信度(→ spec 07 §2)。
- [x] 支持批量与逐块回退,部分失败不覆盖原文,全失败显示错误;独立取消和会话有效性检查(→ spec 07 §3)。

### 6.2 颜色采样
- [x] 四边内缩 2 物理像素,R/G/B 中位数、黑白前景色和批量采样(→ spec 07 §4)。

### 6.3 overlay 窗口
- [x] 运行时透明/无边框/置顶/不夺焦/不占任务栏/鼠标穿透(→ spec 07 §5)。
- [x] 捕获裁剪区域、负坐标和来源屏比例定位;新触发关闭旧层(→ spec 07 §5.2)。
- [x] show/close/payload/ready 命令及类型契约(→ spec 07 §7)。

### 6.4 前端渲染
- [x] managed payload + ready 握手取代首条 init 事件,避免首帧竞态(→ spec 07 §6.1)。
- [x] bbox 相对定位、canvas 8~32px 字号适配、溢出裁剪省略(→ spec 07 §6.2~6.3)。

### 6.5 关闭交互
- [x] 临时 Esc,关闭后仅解绑本流程拥有的注册;冲突时托盘「关闭覆盖层」兜底(→ spec 07 §6.4)。
- [x] `overlay://closed { id: "overlay" }`,新截图前也关闭旧覆盖层。

### 6.6 验收(M6 结束)
- [ ] 框选到显示 ≤4s、颜色采样/文本布局性能。
- [ ] macOS/Windows 真窗口鼠标穿透、不夺焦、Esc 关闭。
- [ ] 多屏混合 DPI/负坐标屏幕对齐 ≤2 逻辑像素。
- [ ] 深浅底文字可读;部分翻译失败与快速重复触发。

---

## M7 · 设置页(W10)

目标:主窗口 UI + 所有配置项完整。**现有设置控件已接通;原生启动/权限/重启验收待做。**

### 7.1 通用 Tab
- [x] 系统/中/英语言,系统/深/浅主题及实时更新(→ spec 09 §2.2)。
- [x] 自启 toggle → LaunchAgent/Windows MSIX StartupTask(无包开发版保留 Run),保存失败回滚(→ spec 10 §3)。
- [x] 启动打开主窗口与真实自动更新检查(→ spec 01 §3、spec 10 §4)。
- [x] 实际监听状态、macOS 权限入口/重新检查;Windows 平台文案。
- [-] 应用内代理已移除(2026-07-07)。

### 7.2 快捷键 Tab
- [x] 显示/录制/预览/清空/冲突反馈和即时重绑;失败保留旧绑定(→ spec 09 §2.4、spec 10 §2)。
- [x] macOS 符号与 Windows Ctrl/Alt/Shift/Win 提示,固定应用内 Cmd/Ctrl+,。

### 7.3 服务 · 文本翻译
- [x] 引擎拖拽排序/启停/增删/设默认,名称/类型/URL/Key/Model/Prompt/Timeout/温度/Token 表单(→ spec 09 §2.3)。
- [x] 测试当前草稿,丢弃过期结果;侧栏仅真实最近测试成功显示绿灯。
- [x] API Key 默认隐藏、可切换显示,已有字段下方明文存储提示满足 MVP 警示(→ spec 01 §6)。
- [x] Apple 翻译桥接;系统版本/平台限制明确显示(→ spec 04 §6.2)。

### 7.4 服务 · OCR
- [x] 按系统显示后端与真实能力;macOS 快速/精准、置信度、13+ 自动检测持久化(→ spec 03)。
- [x] BCP-47 添加/删除/拖动/箭头排序和优先顺序保存(→ spec 01 §2)。
- [x] Windows 系统语言回退、安装语言枚举和系统安装入口;缺语言包跳转 OCR 页(→ spec 03 §4.1)。

### 7.5 关于 / 搜索
- [x] 实际版本/构建日期/Bundle ID/OCR 后端,GitHub/Release/更新检查入口(→ spec 10 §7)。
- [x] MIT 全文和离线第三方声明,按依赖选择全文;依赖变更刷新生成资源。
- [x] 中英关键词搜索、键盘选择、导航/滚动/聚焦(→ spec 09 §2.2)。

### 7.7 验收(M7 结束)
- [x] typecheck/build 和设置纯逻辑回归测试。
- [x] 隔离 IPC 浏览器验证配置编辑和重载、主要控件交互;不修改真实配置。
- [ ] 完整桌面应用重启后所有偏好保存,自启登录、系统授权和快捷键注册实机验收。

---

## M8 · 完善(W11)

目标:主题、i18n、自启、更新等全部打磨到可发布。

### 8.1 国际化
- [ ] 所有 UI 文字抽到语言包(禁止硬编码)(→ spec 09 §6.4)
- [ ] `navigator.language` 前缀匹配(→ spec 09 §6.4)
- [ ] 中文语言包完整;英文语言包完整(→ spec 09 §6.4)

### 8.2 主题
- [ ] CSS 变量统一(`--bg`, `--text`, `--accent`, ...)(→ spec 09 §6.1)
- [ ] 系统主题变化监听 → emit `config://theme-changed` → 所有窗口响应(→ spec 10 §6)
- [ ] macOS vibrancy / Windows acrylic(popup)(→ spec 09 §3.2)

### 8.3 检查更新
- [x] GitHub Releases API + semver,不依赖未发布的 updater 签名端点(→ spec 10 §4)。
- [x] 启动 3s/24h/重新开启检查,托盘和关于页手动检查。
- [x] 版本与 changelog/浏览器 Release 入口,明确无发布与网络错误反馈。
- [-] updater 插件与 latest.json:当前只在浏览器打开正式 Release,无需此链路。

### 8.4 代理联动(已移除)
- [-] 应用内代理功能已移除(2026-07-07),本节取消(→ spec 04 §7)

### 8.5 日志
- [ ] 日志轮转(5MB × 3 份)(→ spec 11 §5)
- [ ] 禁止记录 API Key / 剪贴板内容(code review 检查点)(→ spec 11 §5)

### 8.6 验收(M8 结束)
- [ ] 中英切换 UI 即时更新(→ spec 09 §7)
- [ ] 系统主题切换后三种窗口(main/popup/overlay)都正确跟随(→ spec 09 §7)
- [ ] 自启开关重启系统后验证生效(→ spec 10 §8)
- [ ] 有新版本弹框,无新版本 toast(→ spec 10 §8)

---

## M9 · 打包发布(W12)

目标:双平台构建 + GitHub Release + 文档。

### 9.1 CI
- [x] `.github/workflows/release.yml`:标签版本校验、双架构 macOS DMG + Windows x64 MSIX,成功后发布 GitHub Release 与 SHA256;分支手动运行可仅构建(→ spec 11 §4.3)。
- [-] macOS universal 单包暂用 Apple Silicon / Intel 两份独立 DMG,覆盖两种架构(→ spec 11 §4.3)。
- [ ] macOS:codesign + notarize + staple(→ spec 11 §4.1)
- [x] Windows:MSIX MakeAppx/SignTool 测试签名构建及安装助手已实现;正式证书和 Windows 实际安装报告待完成(→ spec 11 §4.2)。
- [ ] 正式产物:`.dmg`(mac)、`.msix`(win);测试产物流程已编写,尚未公开发布(→ spec 11 §3.1)。
- [-] `latest.json` 无需生成:当前使用正式 GitHub Release tag 检查版本(→ spec 10 §4.3)
- [x] v0.1.0 Release notes 手动整理,写明平台、签名和未验收范围(→ docs/releases/v0.1.0.md)。

### 9.2 包体
- [x] Rust release:`opt-level = "s"`、去掉调试信息(→ spec 11 §1.2)。
- [ ] 前端生产构建 minify + treeshake + shadcn/ui 按需加载(→ spec 11 §1.2)
- [ ] `.dmg` / `.msix` ≤ 15MB(→ spec 11 §7)

### 9.3 文档
- [ ] `README.md`:中文主文档 + English 版本(→ PRD §8)
- [ ] 截图/GIF 演示 F1/F2/F3(→ PRD §8)
- [ ] 开发文档:本地跑起来的步骤(→ PRD §8)
- [ ] Issue / PR 模板(→ PRD §8)
- [ ] 贡献指南(CONTRIBUTING.md)

### 9.4 验收(M9 结束)
- [ ] macOS 通过 Gatekeeper(公证成功,首次双击打开无警告)(→ spec 11 §7)
- [ ] Windows MSIX 签名可信、安装后 OCR 诊断通过;正式证书/签名尚未准备(→ spec 11 §7)。
- [ ] 首个 GitHub Release 发布;updater 验证可识别到新版本(→ spec 10 §4)
- [ ] 抓包确认只有翻译 API + GitHub 更新请求,无其他外联(→ spec 11 §7)
- [ ] 冷启动 ≤ 2s;空闲内存 ≤ 150MB(M1/i7 均测)(→ spec 11 §7)

---

## 持续执行(贯穿各里程碑)

### 代码质量
- [x] `cargo check` / `pnpm tsc --noEmit` 每次都过 —— 注:`cargo clippy` 未常态化,M9 前补
- [x] Rust 侧禁止 `unwrap()` / `expect()` 越出 setup(边界处手工 `map_err`)(→ spec 00 §6)
- [x] 所有 Tauri command 走白名单(capabilities),前端无文件系统直写权限(→ spec 11 §2.2)

### 跨平台验证
- [ ] 每个功能里程碑结束前,双平台各跑一遍核心路径
- [ ] 多屏 + HiDPI 在 M2/M4/M6 结束时做回归(→ spec 02 §8, spec 07 §10)

### 风险观察项 ⚠️
- [x] Vision / Win OCR 桥接可行性(→ PRD §6.2) —— macOS Vision **GO**(headless 验证通过);Windows 待环境
- [ ] 划词监听权限 / 稳定性(macOS 辅助功能 / Windows UIA)(→ spec 05 §2)
- [ ] 原图覆盖 ESC 快捷键与系统/其他应用冲突(→ spec 07 §11)
- [ ] LLM 批量翻译 JSON 稳定性(多模型回归测试)(→ spec 04 §11)
- [ ] macOS 公证流程(早期跑完整 CI 验证)(→ spec 11 §4.1)
- [ ] Windows `transparent + fullscreen + alwaysOnTop` 组合黑屏(→ spec 02 §9)

### 隐私/安全自检
- [ ] 抓包检查:无非预期外联(→ spec 11 §2.1)
- [ ] 日志检查:无 API Key / 剪贴板内容(→ spec 11 §5)
- [ ] 截图位图不落盘(code review 检查)(→ spec 11 §2)

---

## P2(MVP 之后,不在本文跟踪)

- 弹框操作:收藏、TTS 朗读(→ PRD §7)
- 翻译缓存(→ PRD §7)
- 流式输出(→ PRD §7)
- OCR 引擎可切换(Tesseract / PaddleOCR)(→ PRD §7,spec 03 §10)
- 插件系统(→ PRD §7)
- 多引擎对比(→ PRD §7)
- 导入导出配置(→ PRD §7)
- 词典模式(→ PRD §7)
- 系统 keychain 存储敏感字段(→ spec 01 §6)
- 应用内自动更新(需完整签名链)(→ spec 10 §9)

---

**文档版本**:v0.5(2026-10-14,设置控件、F3、Windows 划词及 OCR MSIX 分发实现;原生安装验收单列)
**上游**:[PRD.md](./PRD.md) v0.3 / [spec/](./spec/) v0.2
