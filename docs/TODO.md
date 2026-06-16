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
- [ ] entitlements 声明 `NSScreenCaptureUsageDescription`(→ spec 11 §4.1)

### 2.4 Windows.Media.Ocr ⏳ 待 Windows 环境
- [ ] `windows` crate 引入 `Graphics.Imaging` + `Media.Ocr` + `Storage.Streams`(→ spec 03 §4)
- [ ] PNG → `SoftwareBitmap` 流水线(→ spec 03 §4)
- [ ] `OcrEngine::TryCreateFromLanguage` + `TryCreateFromUserProfileLanguages` fallback(→ spec 03 §4)
- [ ] Line bbox = Word bbox 并集;置信度统一填 0.9(→ spec 03 §4)
- [ ] 无语言包 → emit `ocr://language-missing`(→ spec 03 §4.1)
- 注:`core/ocr/windows.rs` 已有 `WinMediaOcr` stub(返回 `OcrUnavailable`),待 Windows 机实现+验证。

### 2.5 坐标验证(可视化调试工具)
- [ ] Debug 面板:输入一张 fixture 图,彩色框叠加显示 block bbox,目视对齐(→ spec 03 §8)
- [ ] 多 DPR fixture 单测:1×、2×、3×(→ spec 02 §8) —— 注:已有 1× fixture 的 OCR 单测;2×/3× 与多 DPR 回归 TODO

### 2.6 兜底计划 ⚠️
- [x] W2 末尾做可行性评估(→ PRD §6.2) —— **结论:GO**,Vision 桥接可行,无需 Tesseract 兜底
- [ ] 在 `core/ocr/` 下为 Tesseract 预留实现位(空 mod,不建构依赖) —— 暂不需要(Vision GO)

### 2.7 验收(M2 结束)
- [x] fixture 图识别,block 数量与人工标注 ±1(→ spec 03 §8) —— 注:macOS Vision headless 单测通过;Windows 待
- [x] 1920×1080 区域 OCR ≤ 1s(→ spec 11 §1) —— 注:实测小图 ~320ms,远低于阈值;大图实测 TODO
- [ ] bbox 在屏幕叠加与原文字对齐 ≤ 2 逻辑像素(→ spec 03 §8) —— 真实屏端到端校准待
- [ ] Windows 无语言包 graceful 返回 `OcrUnavailable`,不崩溃(→ spec 03 §8) —— stub 代码层已 graceful;Windows 未运行验证

---

## M3 · 翻译引擎(W4)

目标:OpenAI 兼容接口可用,配置体系完整。

### 3.1 抽象与管理
- [ ] `Translator` trait(→ spec 04 §1)
- [ ] `EngineKind` 枚举(→ spec 04 §1)
- [ ] `TranslatorManager`(含共享 `reqwest::Client` 与 `default_id`)(→ spec 04 §2)
- [ ] `reload(configs, proxy)` 支持热更新(不重启)(→ spec 04 §2)
- [ ] 订阅 `config://changed` 触发 reload(→ spec 01 §4.3)

### 3.2 OpenAI 兼容
- [ ] 单条翻译(chat completions 非流式)(→ spec 04 §3.1)
- [ ] 默认 system prompt + `{sourceLang}`/`{targetLang}` 变量替换(→ spec 04 §3.1)
- [ ] 批量翻译(JSON 数组)+ 解析兜底(失败降级为逐条串行)(→ spec 04 §3.2)
- [ ] 超时字段尊重(`engine.timeoutMs`)

### 3.3 传统 API(P0 但可分批)
- [ ] Google Translate v2(批量 `q[]`)(→ spec 04 §4)
- [ ] DeepL(Free/Pro URL 切换 + 语言代码映射 `zh-CN` → `ZH`)(→ spec 04 §5)
- [ ] 有道智云(HMAC-SHA256 签名,`appKey:appSecret` 拆分)(→ spec 04 §6)

### 3.4 代理
- [ ] `build_http_client(proxy)`(→ spec 04 §7)
- [ ] `engine.useProxy = false` 时用无代理 client(本地 LLM 不走代理)(→ spec 04 §7)

### 3.5 Tauri commands
- [ ] `translate_text` / `translate_batch`(→ spec 04 §8)
- [ ] `test_engine`(发送 "Hello" → 返回延迟/结果)(→ spec 04 §8)
- [ ] `list_engines`(→ spec 01 §5)

### 3.6 错误映射
- [ ] 超时 → `Network("timeout")`(→ spec 04 §9)
- [ ] 4xx → `TranslateFailed("auth"|"...")`;5xx → `"server_error"`(→ spec 04 §9)
- [ ] 解析失败 → `"parse_error"`(→ spec 04 §9)

### 3.7 验收(M3 结束)
- [ ] LM Studio / Ollama 本地端点接入,一段英文 → 中文端到端 OK(→ spec 04 §10)
- [ ] 批量 10 条单次请求返回,长度一致(→ spec 04 §10)
- [ ] 设置页"测试连接"按钮 ≤ 3s 返回结果(→ spec 04 §10)
- [ ] 代理启用后通过 Charles/mitmproxy 验证流量走代理(→ spec 10 §8)
- [ ] 引擎配置热更新后,下一次翻译使用新配置无需重启(→ spec 04 §10)

---

## M4 · 截图翻译 F2(W5-6)

目标:F2 全流程可用(截图 → OCR → 翻译 → 弹框)。

### 4.1 选区蒙层
- [ ] 运行时创建 `selection` 窗口覆盖全虚拟屏(→ spec 02 §3.1 / spec 09 §4.1)
- [ ] 前端 canvas 实现:半透遮罩 + 镂空 + 选区边框 + 尺寸提示(→ spec 02 §3.2 / spec 09 §4.2)
- [ ] 鼠标事件:down/move/up 绘制;ESC 取消(→ spec 02 §3.2)
- [ ] `clientX/Y` + 窗口 `innerPosition()` → 虚拟屏逻辑坐标(→ spec 09 §4.3)
- [ ] emit `capture://region-selected { x,y,w,h, displayId, trigger }`(→ spec 02 §3.3)
- [ ] `w<4 || h<4` 视为点击取消,emit `capture://region-cancelled`(→ spec 02 §6)
- [ ] `start_selection` / `cancel_selection` commands(→ spec 02 §4)

### 4.2 F2 流程编排
- [ ] 全局快捷键 `Alt+S` → `start_selection(Screenshot)`(→ spec 06 §2.1)
- [ ] 接收 `capture://region-selected` → 销毁选区窗口 → 执行采集(→ spec 06 §2.2)
- [ ] capture → OCR(按 `preferred_languages` + `min_confidence`)(→ spec 06 §2.3, §2.4)
- [ ] OCR block 拼接(同行空格,跨行 `\n`)→ 送翻译(→ spec 06 §2.4)
- [ ] OCR 为空 → emit `screenshot://error { stage: "ocr", code: "no_text" }`(→ spec 06 §2.4)
- [ ] 翻译完成 → emit `screenshot://result { ... }`(→ spec 06 §2.6)

### 4.3 进度与取消
- [ ] 关键节点 emit `screenshot://progress { stage }`(→ spec 06 §5)
- [ ] `CancellationToken` per-session;新触发取消旧 token(→ spec 06 §7)
- [ ] ESC 全局取消(任意阶段)(→ spec 06 §6)
- [ ] 总超时 10s → `screenshot://error`(→ spec 06 §4)

### 4.4 popup 弹框
- [ ] popup 窗口静态配置(常驻隐藏)(→ spec 09 §3.1)
- [ ] 布局:语言行 / 原文(可折叠)/ 译文 / 工具栏(→ spec 09 §3.2)
- [ ] 状态:loading(骨架屏)/ success / error(→ spec 09 §3.3)
- [ ] 监听 `screenshot://result` 与 `screenshot://error` → 渲染(→ spec 06 §3)
- [ ] 动态高度 `window.setSize` 按内容调整(→ spec 09 §3.1)
- [ ] 定位:选区下方 `y + h + 12`,水平居中;溢出则夹紧(→ spec 06 §3)
- [ ] 点击窗口外/ESC → 隐藏(→ spec 09 §3.4)
- [ ] 复制译文按钮(→ spec 09 §3.4)

### 4.5 权限引导
- [ ] macOS 屏幕录制权限缺失 → popup 展示引导 + "打开系统设置"按钮(→ spec 06 §4)

### 4.6 验收(M4 结束)
- [ ] Alt+S → 进入截图模式 ≤ 300ms(→ spec 06 §8)
- [ ] 框选 → 弹框占位出现 ≤ 500ms(→ spec 06 §8)
- [ ] 框选 → 完整结果 ≤ 3s(本地 OCR + 本地 LLM)(→ spec 06 §8)
- [ ] ESC 任意阶段可取消,不残留窗口(→ spec 06 §8)
- [ ] 多屏 + 不同 DPR 选区准确(→ spec 02 §7)
- [ ] 连续 Alt+S(未完成先前)→ 新会话正确取代旧会话(→ spec 06 §9)

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
- [ ] 快捷键触发:读剪贴板 → emit `selection://triggered`(source: "shortcut")(→ spec 05 §3)
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
- [ ] 无选中文本时按快捷键 → toast 提示(→ spec 05 §10)
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
- [ ] 左列:引擎列表(拖拽排序、enabled toggle、删除按钮)(→ spec 09 §2.3) —— 注:列表 UI(状态点/默认徽章/高亮)已有;拖拽/删除待
- [ ] 右列:详情表单(名称/类型/URL/Key/Model/System Prompt/Timeout/UseProxy)(→ spec 09 §2.3) —— 注:展示态已有;可编辑表单待
- [ ] 设为默认引擎按钮 → 更新 `translator.defaultEngineId`(→ spec 01 §2)
- [ ] "测试连接"按钮 → `test_engine`(→ spec 04 §8) —— 注:按钮 UI 已有;接线待 M3 引擎就绪
- [ ] 添加引擎按钮 → 空表单;保存前校验必填字段(→ spec 09 §2.3)
- [ ] API Key 输入默认 `type="password"` + 显示 toggle(→ spec 01 §6)
- [ ] 敏感字段明文保存的警示条(→ spec 01 §6)

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
