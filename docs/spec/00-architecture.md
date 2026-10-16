# Spec 00 · 总览与架构

> 覆盖 PRD §3(技术方案)。定义模块划分、进程/窗口模型、前后端通信与共享类型约定。后续各模块 spec 在此之上展开。

---

## 1. 目标

- 一个主进程(Rust)+ 多个 WebView 窗口(React)的 Tauri 应用。
- 功能沿 "**采集 → 识别 → 翻译 → 展示**" 主干切分,各层通过 trait 抽象,未来可替换实现。
- 所有平台差异(OCR、划词监听、贴靠窗口)收拢到 `core/` 模块,上层 Tauri command 只面向统一接口。

## 2. 模块分层

```
                 ┌─────────────── Frontend (React) ───────────────┐
                 │  main  │  popup  │  selection  │  overlay      │
                 └───────────────┬─────────────┬──────────────────┘
                                 │ invoke      │ emit/listen
                 ┌───────────────▼─────────────▼──────────────────┐
                 │                 Tauri commands                  │
                 │  translate / ocr / capture / config             │
                 └───────────────┬─────────────┬──────────────────┘
                                 │             │
                 ┌───────────────▼──┐       ┌──▼──────────────┐
                 │   core (业务)    │       │   config 存储   │
                 │ capture│ocr      │       │   JSON 持久化   │
                 │ translator│overlay│      └─────────────────┘
                 │ selection        │
                 └─────┬──────┬─────┘
                       │      │
                 ┌─────▼──┐ ┌─▼────────────┐
                 │ 平台桥 │ │ 外部依赖     │
                 │ macOS  │ │ reqwest / LLM│
                 │ Windows│ │ OCR API      │
                 └────────┘ └──────────────┘
```

### 2.1 Rust 侧目录(在 PRD §3.3 基础上细化)

```
src-tauri/src/
├── main.rs              # Tauri::Builder 装配
├── setup.rs             # 启动阶段:加载配置、注册快捷键、建托盘
├── types.rs             # 跨模块共享类型(+ ts-rs 导出)
├── error.rs             # 统一错误枚举 LenzError
├── commands/            # 前端可调用入口,仅做参数校验与转发
│   ├── mod.rs
│   ├── translate.rs
│   ├── ocr.rs
│   ├── capture.rs
│   ├── overlay.rs
│   └── config.rs
├── core/
│   ├── capture.rs       # 屏幕位图获取(xcap 包装)
│   ├── ocr/             # Spec 03
│   ├── translator/      # Spec 04
│   ├── overlay.rs       # Spec 07
│   ├── selection.rs     # Spec 05
│   └── shortcut.rs      # 全局快捷键注册/解绑
└── config/              # Spec 01
    ├── mod.rs
    ├── schema.rs
    └── migrate.rs       # 未来版本迁移
```

### 2.2 前端目录

```
src/
├── main.tsx             # 根据 URL 参数路由到不同窗口入口
├── windows/
│   ├── main/            # 设置主窗口
│   ├── popup/           # 翻译结果弹框
│   ├── selection/       # 截图蒙层
│   └── overlay/         # 原图覆盖层
├── components/          # 通用 UI(shadcn/ui 二次封装)
├── hooks/               # useConfig / useTranslate / useInvoke 等
├── stores/              # Zustand:config, session
├── lib/
│   ├── ipc.ts           # invoke/emit 薄封装 + 类型注入
│   ├── types.ts         # 从 Rust 导出的类型
│   └── i18n.ts
└── styles/
```

## 3. 窗口模型

Lenz 运行时最多存在四类窗口,职责单一,互不依赖:

| 窗口 label | 来源 | 生命周期 | 可见性 | 备注 |
|---|---|---|---|---|
| `main` | 静态(tauri.conf) | 常驻(可隐藏) | 用户主动打开/从托盘 | 设置页 |
| `popup` | 静态(tauri.conf) | 常驻(默认隐藏) | 翻译触发时 show+定位 | 复用同一实例,避免闪烁 |
| `selection` | 运行时创建 | 一次性 | 进入截图模式时 | 关闭后销毁 |
| `overlay` | 运行时创建 | 一次性 | F3 原图覆盖时 | 关闭后销毁,支持多屏多实例 |

详见 [Spec 09](./09-windows-ui.md)。

## 4. 前后端通信

### 4.1 调用约定

- **请求-响应**:前端 `invoke("xxx", payload)` → Rust `#[tauri::command] async fn xxx(...)`。
- **后端主动推送**:Rust `app.emit("event-name", payload)` → 前端 `listen("event-name", ...)`。

### 4.2 命令清单(索引,细节在各模块 spec)

| Command | 模块 | 说明 |
|---|---|---|
| `get_config` / `set_config` / `reset_config` | config | Spec 01 |
| `list_engines` / `test_engine` | config+translator | Spec 01/04 |
| `translate_text` / `translate_batch` | translator | Spec 04 |
| `capture_screen_region` | capture | Spec 02 |
| `start_selection` / `cancel_selection` | capture | Spec 02 |
| `run_ocr` | ocr | Spec 03 |
| `show_overlay` / `close_overlay` | overlay | Spec 07 |
| `toggle_selection_listener` | selection | Spec 05 |
| `check_update` | system | Spec 10 |

### 4.3 事件清单

| Event | 发起方 | 载荷 | 用途 |
|---|---|---|---|
| `shortcut://selection` | Rust | — | 划词快捷键触发 |
| `shortcut://screenshot` | Rust | — | 截图快捷键触发 |
| `shortcut://overlay` | Rust | — | 原图覆盖快捷键触发 |
| `selection://text-changed` | Rust | `{ text, lang? }` | 自动划词监听捕获到新选区 |
| `capture://region-selected` | selection 窗口 | `{ x, y, w, h, displayId }` | 用户完成框选 |
| `overlay://closed` | overlay 窗口 | `{ id }` | 覆盖层关闭 |
| `config://changed` | Rust | 变更 keypath | 通知所有窗口刷新配置 |

## 5. 共享类型(片段)

放在 `src-tauri/src/types.rs`,通过 `ts-rs` 或 `specta` 同步到前端 `src/lib/types.ts`。以下是跨模块共享的核心类型:

```rust
#[derive(Serialize, Deserialize, Clone)]
pub struct Rect { pub x: i32, pub y: i32, pub w: i32, pub h: i32 }

#[derive(Serialize, Deserialize, Clone, Copy)]
pub enum TriggerKind { Selection, Screenshot, Overlay }

#[derive(Serialize, Deserialize, Clone)]
pub struct TranslateRequest {
    pub texts: Vec<String>,
    pub source_lang: Option<String>,
    pub target_lang: String,
    pub engine_id: Option<String>, // None = 默认引擎
}

#[derive(Serialize, Deserialize, Clone)]
pub struct TranslateResponse {
    pub translations: Vec<String>,
    pub detected_source_lang: Option<String>,
    pub engine_id: String,
    pub cost_ms: u32,
}

#[derive(Serialize, Deserialize)]
#[serde(tag = "code", content = "message")]
pub enum LenzError {
    Config(String),
    OcrUnavailable(String),
    OcrFailed(String),
    TranslateFailed(String),
    Network(String),
    Permission(String),
    Internal(String),
}
```

## 6. 错误处理策略

1. Rust 侧统一使用 `Result<T, LenzError>`,边界处手工 `map_err`。
2. Tauri command 返回 `Result<T, LenzError>`,前端 `invoke` Promise reject 对应 error。
3. 前端 `lib/ipc.ts` 统一拦截 reject,按 `code` 分发:
   - `Permission` → 引导用户去系统设置授权。
   - `Network` → toast 重试。
   - 其他 → toast 展示 message,记录到本地日志。
4. 禁止在 command 内 `unwrap()` / `expect()`,一律返回 error code。

## 7. 启动时序

```
startup plugin（配置窗口创建前）:
  1. 加载 config(容错:损坏则备份 + 重建)
     初始化全部 managed state、配置订阅及窗口事件监听
Tauri 从配置自动创建 main/popup/indicator WebView
app.setup():
  2. 注册托盘图标与菜单
  3. 注册全局快捷键(config 驱动)
  4. 按 config.selection.autoDetect 启动划词监听
  5. 显示 main 窗口(若 config.app.openOnLaunch、Windows --show-main 或托盘创建失败)
  6. 异步触发 check_update(若 config.app.checkUpdate)
```

失败 step 不阻断主流程:记录到日志并 emit `app://startup-warning`。

WebView2 创建窗口时会泵送消息,已有前端可能立即发送 IPC。全部命令所需的状态必须在建窗前注入,避免 `state() called before manage()` 导致启动期进程退出。配置与建窗等致命失败无法继续时,Windows 记录独立启动日志并显示原生错误提示。

## 8. 验收标准

- `cargo check` + `pnpm tsc --noEmit` 均通过。
- 任一子模块移除后,编译依然能通过总览层的接口约束(trait 存在但无实现)。
- 从前端无法绕过 command 直接访问文件系统或系统 API(Tauri 白名单限制)。

## 9. 开放问题

- [ ] ts-rs vs specta:二者择一,取决于 Tauri 2 当前生态兼容度,W1 定型。
- [ ] 是否引入单独的后台线程池(tokio `spawn_blocking`)用于 OCR,还是复用 Tauri 默认 runtime。待 M2 压测后决定。
