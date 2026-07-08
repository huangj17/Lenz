# Spec 01 · 配置与存储

> 覆盖 PRD §2.2 F4(翻译引擎配置)、§2.3(设置页)以及跨模块的持久化约定。定义配置 schema、读写时序与迁移策略。

---

## 1. 存储位置

| 平台 | 目录 | 文件 |
|---|---|---|
| macOS | `~/Library/Application Support/app.lenz.desktop/` | `config.json` |
| Windows | `%APPDATA%/Lenz/` | 同上 |

通过 `tauri::api::path::app_config_dir` 获取基目录;首次启动不存在则创建。

## 2. 配置 Schema

```typescript
// TypeScript 表示(Rust 端用 serde derive 保持同构)
interface Config {
  version: number;                  // schema 版本号,用于迁移
  app: {
    locale: "system" | "zh-CN" | "en-US";
    theme: "system" | "light" | "dark";
    autoStart: boolean;
    checkUpdate: boolean;
    openOnLaunch: boolean;          // 启动时是否展示主窗口
  };
  shortcuts: {
    selection: string | null;       // 如 "Alt+D",null 表示禁用
    screenshot: string | null;
    overlay: string | null;
    openMain: string | null;
  };
  selection: {
    enabled: boolean;               // 总开关
    autoDetect: boolean;            // 是否启用自动弹框
    popupDelayMs: number;           // 选中后延迟触发,防抖
  };
  ocr: {
    engine: "system";               // MVP 仅系统
    preferredLanguages: string[];   // BCP-47
    minConfidence: number;          // 0~1
  };
  translator: {
    defaultEngineId: string;
    targetLang: string;             // 默认 "zh-CN"
    engines: TranslatorEngine[];
  };
  // 注:应用内代理功能已移除,Config 中没有 `proxy` 字段(见 spec 04 §7 / spec 10 §5)。
  ui: {
    popup: {
      fontSize: number;             // 默认 14
      showOriginal: boolean;
    };
  };
}

interface TranslatorEngine {
  id: string;                       // uuid
  name: string;                     // 用户自定义
  kind: "openai" | "google" | "deepl" | "youdao";
  enabled: boolean;
  apiUrl: string;
  apiKey: string;
  model?: string;                   // LLM 类必填
  systemPrompt?: string;            // LLM 类,支持 {sourceLang}/{targetLang}
  timeoutMs: number;                // 默认 15000
}
```

## 3. 默认值

首次启动生成的默认 config:

- `version = 1`
- `app.locale = "system"`, `theme = "system"`
- `shortcuts` 按 PRD §4.1 平台默认值填充
- `selection.autoDetect = false`(避免误触;用户按需开启)
- `translator.engines = []`,用户需手动添加第一个引擎

## 4. 读写时序

### 4.1 启动加载

```
load_config():
  path := app_config_dir / config.json
  if !exists: write_default(); return default
  raw := fs::read_to_string(path)?
  parsed := serde_json::from_str::<Config>(raw)
  if parsed.version < CURRENT: parsed = migrate(parsed)
  return parsed
```

- 解析失败:备份损坏文件为 `config.json.bak.<timestamp>`,写默认配置,emit `config://corrupted`。

### 4.2 写入

- 所有写入通过 `set_config` command 发起,内部用 **RwLock 包裹的 Config 实例**:
  1. 获取写锁
  2. 应用 patch(深合并或直接替换)
  3. 序列化到 `config.json.tmp`
  4. `rename` 到 `config.json`(原子写)
  5. emit `config://changed` 指示变更 keypath(如 `["shortcuts.selection"]`)

### 4.3 变更副作用

监听 `config://changed` 的副作用由各模块订阅:

| keypath | 副作用 |
|---|---|
| `shortcuts.*` | 重新注册全局快捷键 |
| `selection.enabled` / `selection.autoDetect` | 启/停划词监听线程 |
| `app.autoStart` | 调用 `tauri-plugin-autostart` |
| `app.theme` / `app.locale` | 通知所有窗口重新渲染 |

## 5. Tauri Commands

```rust
#[tauri::command]
async fn get_config(state: State<'_, ConfigStore>) -> Result<Config, LenzError>;

#[tauri::command]
async fn set_config(
    state: State<'_, ConfigStore>,
    patch: serde_json::Value,      // JSON Merge Patch(RFC 7396)
    app: AppHandle,
) -> Result<Config, LenzError>;

#[tauri::command]
async fn reset_config(state: State<'_, ConfigStore>, app: AppHandle) -> Result<Config, LenzError>;

#[tauri::command]
async fn list_engines(state: State<'_, ConfigStore>) -> Result<Vec<TranslatorEngine>, LenzError>;

#[tauri::command]
async fn test_engine(engine: TranslatorEngine) -> Result<TestEngineResult, LenzError>;

struct TestEngineResult { ok: bool, latency_ms: u32, sample: String, error: Option<String> }
```

前端封装:

```ts
// src/lib/ipc.ts
export const configApi = {
  get: () => invoke<Config>("get_config"),
  patch: (patch: DeepPartial<Config>) => invoke<Config>("set_config", { patch }),
  reset: () => invoke<Config>("reset_config"),
};
```

## 6. 敏感字段处理

- API Key 以**明文**保存在 JSON 文件中(MVP)。
- 设置页 UI 输入框 `type="password"`,需要显式点击"显示"才展示。
- 在关于/设置页顶部给出提示:"凭据以明文保存在本机;请勿共享 config.json。"
- 未来(P2)接入系统 keychain(`keyring` crate)。在此之前,schema 中的字段名不变,以便零迁移切换。

## 7. 迁移策略

- `migrate(old: Config) -> Config`:按 `version` 字段分支,逐级提升。
- 每次 schema 变更需同时提交:
  1. 新版 struct 定义(`config/schema.rs`)
  2. 迁移函数(`config/migrate.rs::migrate_v1_to_v2`)
  3. 单元测试:旧 fixture → 迁移后等价性

## 8. 验收标准

- [ ] 首次启动能生成完整默认 config,所有必填字段非空。
- [ ] 设置页修改任一配置,进程重启后生效。
- [ ] 手动破坏 config.json(随机字节),应用仍能启动,损坏文件被备份。
- [ ] 快捷键字段修改后立即重绑,无需重启。
- [ ] config 文件大小在仅 1 个引擎时 < 2KB。

## 9. 测试点

- 并发写入:两次 `set_config` 并发,最终文件内容为后者合并结果(靠写锁保证)。
- 断电/进程被杀:由于 `rename` 原子写,不会出现半写文件。
- 跨平台路径:Windows `%APPDATA%` 含中文用户名时正常。
