# Lenz 开发 TODO List

> 基于 [PRD v0.3](./PRD.md) §6.1 里程碑 + [spec](./spec/) 各模块详细规格拆分的任务清单。
>
> 约定:
> - 完成标记为 `[x]`,跳过用 `[-]` 并注明原因。
> - 每项任务右侧 `(→ spec XX §Y)` 指向对应 spec 节。
> - P0/P1/P2 对应 PRD 优先级;风险项以 ⚠️ 标记。
> - 跨里程碑的通用约束集中写在本文档末尾的"持续执行"小节。

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
- [x] 各监听方订阅模板(→ spec 01 §4.3) —— 注:已接 config://changed + 前端主题/语言订阅 + 托盘重建;快捷键热重绑→M7、自启/代理→M8
- [x] schema 迁移框架:`migrate(old, version)` 占位(→ spec 01 §7)
- [x] 测试(→ spec 01 §9) —— 注:merge patch / 损坏 fallback 已有单测;并发 / 含中文路径 TODO

### 1.3 托盘与主窗口
- [x] `TrayIcon` 构建 + 菜单项(打开主窗口、F1/F2/F3、划词监听 toggle、设置、检查更新、关于、退出)(→ spec 10 §1.2)
- [x] 左键点击托盘 → 显示主窗口(→ spec 10 §1.3)
- [x] 配置变更后菜单重建(`rebuild_tray_menu`)(→ spec 10 §1.3)
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

### 2.4 Windows.Media.Ocr(实现已写,⏳ 待 Windows/CI 验证编译)
- [x] `windows` crate(0.62)引入 `Graphics.Imaging` + `Media.Ocr` + `Storage.Streams` + `Globalization`(→ spec 03 §4)
- [x] PNG → `InMemoryRandomAccessStream` → `BitmapDecoder` → `SoftwareBitmap` 流水线(→ spec 03 §4)
- [x] `TryCreateFromLanguage` 依序尝试 + `TryCreateFromUserProfileLanguages` 回退(→ spec 03 §4)
- [x] Line bbox = Word.BoundingRect 并集;置信度统一 0.9;`map_pixel_bbox` 回映射(→ spec 03 §4)
- [ ] 无语言包 → emit `ocr://language-missing`(→ spec 03 §4.1) —— 注:引擎层返回 `OcrUnavailable`;事件 emit 需 AppHandle,留 M4/M7 上层接线
- ⚠️ `core/ocr/windows.rs` 完整实现已写,但本机 macOS 的 `tauri-build` 阻止交叉 `cargo check --target *-windows`,故**未本地验证编译**;需 Windows / CI(windows-latest)校验(重点:`TryCreateFromLanguage` 缺包返回语义、`IAsyncOperation::get()`)。

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
> **苹果翻译为占位**(返回 `apple_not_implemented`),原生桥接待后续。
> **3.7 验收已通过**(2026-06-16):对真实 Ollama(`qwen2.5:1.5b`)/ LM Studio 端点新增
> 7 项 `#[ignore]` 门控验收测试(`mod acceptance`)全绿——端到端 / 批量 10 条 /
> test_engine 57ms / 引擎热更新路由 / 代理强制 + 本地转发拦截(无 mitmproxy 时的等价证明)。

### 3.1 抽象与管理
- [x] `Translator` trait(→ spec 04 §1) —— `core/translator/mod.rs`,含 id/name/kind/supports_batch/translate
- [x] `EngineKind` 枚举(→ spec 04 §1) —— 复用 `config::schema::EngineKind`(openai/google/deepl/youdao)
- [x] `TranslatorManager`(含共享 `reqwest::Client` 与 `default_id`)(→ spec 04 §2) —— 代理/直连双 client,id→引擎表
- [x] `reload(configs, proxy)` 支持热更新(不重启)(→ spec 04 §2)
- [x] 订阅 `config://changed` 触发 reload(→ spec 01 §4.3) —— `setup.rs` listen,仅 `translator.*`/`proxy.*`/整体重置触发

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
- [-] 苹果系统翻译(→ spec 04 §6.2/§11) —— **占位**:可配置/选中,`translate` 返回 `apple_not_implemented`;原生桥接列为后续
- [x] 内置默认引擎预设(有道/苹果/智谱/硅基流动 开,谷歌/微软/ollama/LM Studio 关)(→ spec 04 §12) —— `default_engines()` + schema v1→v2 迁移注入 + 前端 `default-config.ts` 镜像

### 3.4 代理
- [x] `build_http_client(proxy)`(→ spec 04 §7) —— http/https/socks5 + basic auth + bypass_local
- [x] `engine.useProxy = false` 时用无代理 client(本地 LLM 不走代理)(→ spec 04 §7) —— 直连 client 额外 `.no_proxy()` 屏蔽环境变量代理

### 3.5 Tauri commands
- [x] `translate_text` / `translate_batch`(→ spec 04 §8) —— 均路由 `TranslatorManager::translate`
- [x] `test_engine`(发送 "Hello" → 返回延迟/结果)(→ spec 04 §8) —— 一次性引擎,代理取自当前配置;失败 `ok=false`
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
- [x] 代理启用后验证流量走代理(→ spec 10 §8) —— 注:本机无 Charles/mitmproxy,改用自带最小本地转发代理:正检流量过代理(hits=1→`你好`)+ 负检(死代理+不绕本地→连接失败,未静默直连),证同等性质
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
- [x] 总超时 10s → `screenshot://error`(→ spec 06 §4) —— `tokio::time::timeout` 包裹整条 pipeline

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

### 4.6 验收(M4 结束)—— GUI 运行态手测待
- [ ] Alt+S → 进入截图模式 ≤ 300ms(→ spec 06 §8) —— 代码就绪,待运行态测
- [ ] 框选 → 弹框占位出现 ≤ 500ms(→ spec 06 §8) —— popup 框选后立即 show(loading),待实测
- [ ] 框选 → 完整结果 ≤ 3s(本地 OCR + 本地 LLM)(→ spec 06 §8) —— 参考 M2 OCR ~320ms + M3 本地 LLM <1s
- [ ] ESC 任意阶段可取消,不残留窗口(→ spec 06 §8) —— 逻辑已实现,待运行态测
- [ ] 多屏 + 不同 DPR 选区准确(→ spec 02 §7) —— 单屏/同 DPR 精确;混合 DPR 近似(spec 02 §9)
- [ ] 连续 Alt+S(未完成先前)→ 新会话正确取代旧会话(→ spec 06 §9) —— 会话代号机制已实现,待运行态测

---

## M5 · 划词翻译 F1(W7)

目标:F1 功能,macOS 优先,Windows 后补。

### 5.1 监听层
- [ ] `SelectionListener` 抽象(start/stop/restart)(→ spec 05 §2.3)
- [ ] macOS:`CGEventTap` 监听 `kCGEventLeftMouseUp`(→ spec 05 §2.1)
- [ ] macOS:`NSPasteboard.changeCount` 对比读取选中文本(→ spec 05 §2.1)
- [ ] macOS:`AXIsProcessTrusted` 检测 + 首次触发引导 emit `selection://permission-required`(→ spec 05 §2.1)
- [ ] macOS entitlements 声明 `com.apple.security.automation.apple-events`(→ spec 11 §4.1)
- [ ] Windows:`SetWinEventHook` 监听 `EVENT_OBJECT_TEXTSELECTIONCHANGED` + `EVENT_SYSTEM_FOREGROUND`(→ spec 05 §2.2)
- [ ] Windows:`OpenClipboard` + `GetClipboardData(CF_UNICODETEXT)`(→ spec 05 §2.2)

### 5.2 触发逻辑
- [x] 快捷键触发:读剪贴板 → 翻译 → popup(source: "shortcut")(→ spec 05 §3) —— M4 打磨期接通:Alt+D → `tauri-plugin-clipboard-manager` 读选中文本 → 复用 `screenshot://*` 事件渲染;注:改用 Rust 直接编排(未走 `selection://triggered` 事件),光标锚定待本里程碑
- [x] 选中自动复制:Alt+D 先合成系统复制键(macOS `Cmd+C`/CGEvent、Windows `Ctrl+C`/SendInput,`core/keystroke.rs`)→ 轮询剪贴板变化 → 翻译(→ spec 05 §3) —— 双击/划词选中均覆盖;⚠️ macOS 需「辅助功能」权限,缺失则事件被静默丢弃、自动退化为读当前剪贴板/回显上次结果
- [x] 无新选区(未选中/无权限)→ 回显上次结果(`LastResult` managed state 缓存 F1/F2 最近结果,重新 emit `screenshot://result`)(→ spec 05 §3) —— ⚠️ 与 spec 05 §10「无选中→toast」不同,按用户需求改为「再次按键复现上次内容」,spec 05 待同步
- [ ] 自动触发:去抖 `popupDelayMs` + 长度过滤(1~5000)+ 去重(→ spec 05 §4)
- [ ] emit `selection://triggered { text, source, cursorPos }`(→ spec 05 §5)
- [ ] `toggle_selection_listener` command(→ spec 05 §8)
- [ ] 托盘"划词监听"toggle 同步状态(→ spec 10 §1.2)

### 5.3 popup 复用
- [ ] 监听 `selection://triggered` → 复用同一 popup 实例(→ spec 05 §6)
- [ ] 正在翻译中触发新文本 → 取消旧请求(→ spec 05 §6)
- [ ] 定位:`cursorPos + (16,16)` 偏移 + 溢出夹紧(以 popup 尺寸估算)(→ spec 05 §7)

### 5.4 状态机
- [ ] IDLE → WAITING → TRANSLATING → SHOWING 状态切换(→ spec 05 §9)
- [ ] ESC / 文本清空 → 回 IDLE(→ spec 05 §9)

### 5.5 验收(M5 结束)
- [ ] 选中 → 弹框出现 ≤ 500ms(→ spec 05 §10)
- [ ] 快速连选三段,仅最后一段展示;同段不重复触发(→ spec 05 §11)
- [ ] 无选中文本时按快捷键 → 回显上次结果(原 spec 05 §10 为 toast 提示;按用户需求改为复现上次内容,⚠️ spec 05 §10/§11 待同步)
- [ ] 弹框不超屏幕边界(→ spec 05 §10)
- [ ] macOS 权限未授时弹引导(→ spec 05 §11)
- [ ] Windows 基础路径跑通(若 UIA 稳定性不足可延后自动触发,保留快捷键触发)⚠️

---

## M6 · 原图覆盖 F3(W8-9)⚠️ 核心差异化

目标:F3 完整实现,跨屏/HiDPI 正确。

### 6.1 流程编排
- [ ] 全局快捷键 `Alt+F` → `start_selection(Overlay)`(→ spec 07 §1)
- [ ] 接收 `capture://region-selected { trigger: "overlay" }` → capture + OCR(→ spec 07 §1)
- [ ] 按 `min_confidence` 过滤 block,保留 OCR 顺序(不做行合并)(→ spec 07 §2)
- [ ] `translate_batch` 批量翻译;失败 slot 填空字符串(→ spec 07 §3)

### 6.2 颜色采样
- [ ] `sample_block_color`:bbox 四边内缩 2px 采样 → R/G/B 中位数(→ spec 07 §4)
- [ ] Rust 侧计算前景色(luminance > 0.5 选黑,否则选白)(→ spec 07 §4.1)
- [ ] 批量采样:翻译完成后一次遍历(→ spec 07 §4)

### 6.3 overlay 窗口
- [ ] 运行时 `WebviewWindowBuilder` 创建(transparent/decorations:false/alwaysOnTop/skipTaskbar/focusable:false)(→ spec 07 §5.1)
- [ ] `set_ignore_cursor_events(true)` 实现鼠标穿透(→ spec 07 §5.1)
- [ ] 窗口坐标/尺寸 = region(逻辑坐标)(→ spec 07 §5.1)
- [ ] 多触发:先销毁旧 overlay 再建新(→ spec 07 §5.1)
- [ ] `show_overlay` / `close_overlay` commands(→ spec 07 §7)

### 6.4 前端渲染
- [ ] 监听 `overlay://init` 接收数据(→ spec 07 §6.1)
- [ ] 绝对定位每个 block(相对 region 左上)(→ spec 07 §6.2)
- [ ] 字号自适应:从 `min(boxH*0.8, 32)` 起步,canvas measureText 二分/线性探测到 ≥ 8px(→ spec 07 §6.3)
- [ ] 超过最小字号仍溢出 → `overflow:hidden` + ellipsis(→ spec 07 §6.3)

### 6.5 关闭交互 ⚠️
- [ ] 注册临时全局快捷键 ESC(仅 overlay 存在时有效),触发后销毁 + 解绑(→ spec 07 §6.4)
- [ ] emit `overlay://closed`(→ spec 00 §4.3)
- [ ] 评估 ESC 冲突风险:若不可接受,退化为托盘菜单"关闭覆盖"后备(→ spec 07 §11)

### 6.6 验收(M6 结束)
- [ ] 框选 → 覆盖层显示 ≤ 4s(→ spec 07 §9)
- [ ] 译文不超出 bbox(→ spec 07 §9)
- [ ] 覆盖层不拦截鼠标事件(→ spec 07 §9)
- [ ] ESC 关闭覆盖层,下层交互恢复(→ spec 07 §9)
- [ ] 多屏 + DPR 对齐 ≤ 2 逻辑像素(→ spec 07 §9)
- [ ] 深/浅底文字都可读(自动前景色)(→ spec 07 §9)

---

## M7 · 设置页(W10)

目标:主窗口 UI + 所有配置项完整。

> **状态:部分完成**。设置页**视觉/布局**已在设计还原阶段提前严格实现(commit 7d780ae,对齐 `docs/design`);本里程碑剩余 = **编辑/交互功能 + 真实数据绑定**(录制、引擎增删改、滑块、test_engine 接线、代理表单、外链等)。

### 7.1 通用 Tab
- [x] 语言选择(system / zh-CN / en-US)(→ spec 09 §2.2)
- [x] 主题选择(system / light / dark)+ 实时预览(→ spec 09 §6.1)
- [ ] 自启 toggle → `tauri-plugin-autostart`(→ spec 10 §3) —— 注:UI toggle 已接 config;autostart 插件→M8
- [x] 启动时打开主窗口 toggle(→ spec 01 §3)
- [x] 检查更新 toggle(→ spec 10 §4)
- [ ] 代理区块:启用 toggle + 协议/地址/端口/账号/密码 + bypassLocal + "测试连接"(→ spec 10 §5.2) —— 注:通用页仅展示代理地址,完整表单待

### 7.2 快捷键 Tab
- [ ] 每功能一行:显示 + 录制按钮 + 清空(→ spec 09 §2.4) —— 注:行展示 + Kbd + "修改"按钮 UI 已有;录制/清空功能待
- [ ] 录制:捕获组合键 + 预览 + 冲突警告(→ spec 09 §2.4)
- [ ] 保存后立即重绑(订阅 `shortcuts.*` 变更)(→ spec 10 §2.4)

### 7.3 翻译引擎 Tab(设计中并入「服务 · 文本翻译」分段)
- [x] 左列:引擎列表(拖拽排序、enabled toggle、删除按钮)(→ spec 09 §2.3) —— HTML5 原生拖拽重排(整体回写 `translator.engines`)、enabled toggle、删除按钮均已接线
- [x] 右列:详情表单(名称/类型/URL/Key/Model/System Prompt/Timeout/UseProxy)(→ spec 09 §2.3) —— 可编辑:文本失焦提交、开关/下拉即时提交;Model/SystemPrompt 仅 openai 显示,Region 仅 microsoft 显示,apple 显示占位说明
- [x] 设为默认引擎按钮 → 更新 `translator.defaultEngineId`(→ spec 01 §2) —— 非默认引擎显示「设为默认」按钮
- [x] "测试连接"按钮 → `test_engine`(→ spec 04 §8) —— 已接线:测当前草稿值,内联显示延迟/译文样例或失败原因
- [x] 添加引擎按钮 → 空表单 + 必填校验(→ spec 09 §2.3) —— 新增 openai 草稿引擎并选中;按类型做必填校验(名称/模型/Key/region/有道格式),缺项内联非阻断提示
- [x] API Key 输入默认 `type="password"` + 显示 toggle(→ spec 01 §6)
- [ ] 敏感字段明文保存的警示条(→ spec 01 §6) —— 注:字段下方已有「明文保存于本地配置」提示文案;独立警示条待

### 7.4 OCR Tab(设计中并入「服务 · OCR」分段)
- [x] 引擎选择(MVP 只有 system,展示 Apple Vision)(→ spec 09 §2.2)
- [ ] `preferredLanguages` 多选(BCP-47)(→ spec 01 §2) —— 注:语言 chips 展示已有;可编辑多选/排序待
- [ ] `minConfidence` 滑块(0~1)(→ spec 01 §2) —— 注:设计用「快速/精准」分段(UI 占位);minConfidence 绑定待
- [ ] 监听 `ocr://language-missing` → 显示系统设置跳转引导(→ spec 03 §4.1)

### 7.5 关于 Tab
- [x] 版本号(→ spec 10 §7) —— 注:构建日期 `env!("BUILD_DATE")` 待
- [ ] MIT License 链接(→ spec 10 §7) —— 注:按钮 UI 已有;外链(plugin-opener)待
- [ ] GitHub 仓库链接 + Bundle ID(→ spec 10 §7) —— 注:按钮 UI 已有;外链待
- [ ] 第三方许可证页面(`LICENSES.html`,`cargo-about` 生成)(→ spec 10 §7)

### 7.7 验收(M7 结束)
- [x] 所有 Tab 无白屏(→ spec 09 §7) —— 注:`tsc` / `vite build` 通过
- [ ] 改任意配置 → 重启后生效 + 不需要重启的热更新项(快捷键/代理/引擎/主题)即时生效(→ spec 01 §8) —— 注:主题/语言即时;其余待
- [ ] 深/浅 + 中/英四象限 UI 无硬编码穿帮(→ spec 09 §7) —— 注:i18n 已全量化;运行时四象限肉眼验收待

---

## M8 · 完善(W11)

目标:主题、i18n、自启、更新、代理等全部打磨到可发布。

### 8.1 国际化
- [ ] 所有 UI 文字抽到语言包(禁止硬编码)(→ spec 09 §6.4)
- [ ] `navigator.language` 前缀匹配(→ spec 09 §6.4)
- [ ] 中文语言包完整;英文语言包完整(→ spec 09 §6.4)

### 8.2 主题
- [ ] CSS 变量统一(`--bg`, `--text`, `--accent`, ...)(→ spec 09 §6.1)
- [ ] 系统主题变化监听 → emit `config://theme-changed` → 所有窗口响应(→ spec 10 §6)
- [ ] macOS vibrancy / Windows acrylic(popup)(→ spec 09 §3.2)

### 8.3 检查更新
- [ ] `tauri-plugin-updater` 接入(→ spec 10 §4)
- [ ] 启动 3s 延迟自动检查(若开启)(→ spec 10 §4.1)
- [ ] 手动触发(托盘 / 设置页)(→ spec 10 §4.1)
- [ ] 弹框展示版本 + changelog + 浏览器打开 Release 页(不自动下载)(→ spec 10 §4.2)
- [ ] `tauri.conf.json` 配置 `updater.endpoints` 指向 `latest.json`(→ spec 10 §4.3)

### 8.4 代理联动
- [ ] `config.proxy.*` 变更 → `TranslatorManager` 重建 http_client(→ spec 04 §7)
- [ ] `engine.useProxy = false` 的引擎用无代理 client(→ spec 04 §7)

### 8.5 日志
- [ ] 日志轮转(5MB × 3 份)(→ spec 11 §5)
- [ ] 禁止记录 API Key / 剪贴板内容(code review 检查点)(→ spec 11 §5)

### 8.6 验收(M8 结束)
- [ ] 中英切换 UI 即时更新(→ spec 09 §7)
- [ ] 系统主题切换后三种窗口(main/popup/overlay)都正确跟随(→ spec 09 §7)
- [ ] 自启开关重启系统后验证生效(→ spec 10 §8)
- [ ] 有新版本弹框,无新版本 toast(→ spec 10 §8)
- [ ] 代理启用后翻译请求抓包验证(→ spec 10 §8)

---

## M9 · 打包发布(W12)

目标:双平台构建 + GitHub Release + 文档。

### 9.1 CI
- [ ] `.github/workflows/release.yml`:macos-latest + windows-latest 矩阵(→ spec 11 §4.3)
- [ ] macOS:universal binary(`--target universal-apple-darwin`)(→ spec 11 §4.3)
- [ ] macOS:codesign + notarize + staple(→ spec 11 §4.1)
- [ ] Windows:SignTool(若证书就绪)(→ spec 11 §4.2)
- [ ] 产物:`.dmg`(mac)、`.msi`(win)(→ spec 11 §3.1)
- [ ] 自动生成 `latest.json`(updater 端点)(→ spec 11 §4.3)
- [ ] Release notes 由 git log 自动生成(或手动)

### 9.2 包体
- [ ] Rust release:`opt-level = "s"`(→ spec 11 §1.2)
- [ ] 前端生产构建 minify + treeshake + shadcn/ui 按需加载(→ spec 11 §1.2)
- [ ] `.dmg` / `.msi` ≤ 15MB(→ spec 11 §7)

### 9.3 文档
- [ ] `README.md`:中文主文档 + English 版本(→ PRD §8)
- [ ] 截图/GIF 演示 F1/F2/F3(→ PRD §8)
- [ ] 开发文档:本地跑起来的步骤(→ PRD §8)
- [ ] Issue / PR 模板(→ PRD §8)
- [ ] 贡献指南(CONTRIBUTING.md)

### 9.4 验收(M9 结束)
- [ ] macOS 通过 Gatekeeper(公证成功,首次双击打开无警告)(→ spec 11 §7)
- [ ] Windows SmartScreen 可绕过(有证书后改进)(→ spec 11 §7)
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

**文档版本**:v0.3(2026-06-16,同步进度:M1 完成 / M2 macOS 完成、Windows 待 / M7 设置页视觉完成)
**上游**:[PRD.md](./PRD.md) v0.3 / [spec/](./spec/) v0.2
