# Spec 04 · 翻译引擎

> 覆盖 PRD §2.2 F4 与 §3.4(翻译引擎抽象)。定义 Translator trait、各引擎实现、多引擎管理。

---

## 1. Trait 定义

```rust
// src-tauri/src/core/translator/mod.rs

#[async_trait::async_trait]
pub trait Translator: Send + Sync {
    fn id(&self) -> &str;
    fn name(&self) -> &str;
    fn kind(&self) -> EngineKind;
    fn supports_batch(&self) -> bool;
    async fn translate(&self, req: TranslateRequest) -> Result<TranslateResponse, LenzError>;
}

#[derive(Clone, Serialize, Deserialize)]
pub enum EngineKind {
    OpenAICompat,   // 线缆值 "openai":LM Studio / Ollama / 智谱 / 硅基流动 / DeepSeek …
    Google,
    DeepL,
    Youdao,
    Microsoft,      // 微软 Azure Translator(需 Key + region)
    Apple,          // macOS 15+ 原生 Translation,见 §6.2
}

pub struct TranslateRequest {
    pub texts: Vec<String>,             // 1 条或多条(F3 批量)
    pub source_lang: Option<String>,    // BCP-47,None = 自动检测
    pub target_lang: String,
}

pub struct TranslateResponse {
    pub translations: Vec<String>,
    pub detected_source_lang: Option<String>,
    pub cost_ms: u32,
}
```

> 单条翻译统一放入 `texts[0]`,方便调用方无需区分单/批。

## 2. 引擎管理器

```rust
pub struct TranslatorManager {
    engines: RwLock<HashMap<String, Arc<dyn Translator>>>,
    default_id: RwLock<String>,
    http_client: reqwest::Client,       // 共享
}

impl TranslatorManager {
    pub fn reload(&self, configs: Vec<TranslatorEngine>, default_id: &str);
    pub fn get(&self, id: Option<&str>) -> Option<Arc<dyn Translator>>;
    pub async fn translate(&self, id: Option<&str>, req: TranslateRequest)
        -> Result<TranslateResponse, LenzError>;
}
```

- 配置变更(`config://changed` 含 `translator.*`)时调 `reload`,不重启进程。
- 所有引擎共享一个 `reqwest::Client`(见 §7)。

### 2.1 弹窗多服务对比

- F1/F2 原生 pipeline 在符合各服务折叠策略的启用列表中优先选择默认服务,否则按设置顺序选择可用服务。在 `screenshot://translating` 中报告实际主引擎 id 和请求 `requestId`;全部折叠跳过时 id 为空、requestId 为 null,不发起原生翻译。前端按设置顺序创建全部启用行,通过 `translate_popup(req, sessionId, requestId)` 请求其余符合策略的服务,避免重复调用原生主服务。
- 每行独立维护展开状态、`rememberCollapse` 与 `skipWhenCollapsed`,保存在 `ui.popup.services[engineId]`。新查询仅恢复勾选“记住”的状态,未记住默认展开;折叠且跳过时为 idle,显示“展开后翻译”。未开启跳过时仍请求并保留结果,展开不重复调用。
- 折叠且跳过时取消该服务的 pending 请求,调用 `cancel_popup_translation(sessionId, requestId)`;展开 idle 行再发起新的 UUID 请求。后端以 session + request id 取消对应 future,取消先于请求注册时也有效;前端检查请求 id 与 epoch,拒绝被取消请求的迟到响应。取消原生主服务不影响额外服务。
- 原生结果/错误只更新对应服务行;其他服务分别显示加载、译文/耗时或错误。配置不完整的已启用服务也保留错误行,不静默消失。禁用服务不请求、不展示。
- 原文编辑、语言切换/互换会先 `begin_popup_session`,再按记忆折叠策略并行请求启用服务。失败行单独重试,保留其余成功结果。关闭/新会话通过共享 `ScreenshotSession` 释放所有旧请求;前端 epoch 和 `PopupSessions` 同时阻止迟到响应/事件。
- 原生缓存回显只含主服务的旧结果;保留其中仍启用的主服务结果,其余当前启用服务按折叠策略重新查询。若缓存服务已经禁用,不展示它。折叠成功项展开时复用缓存。
- `translate_text` / `translate_batch` 的默认路由与原图覆盖使用默认服务;弹窗折叠策略不修改全局默认服务。

## 3. OpenAI 兼容实现

文件:`core/translator/openai.rs`

适用:LM Studio、Ollama、DeepSeek、OpenAI 官方。

### 3.1 单条翻译

```
POST {apiUrl}/v1/chat/completions
Authorization: Bearer {apiKey}
Content-Type: application/json

{
  "model": "{model}",
  "messages": [
    { "role": "system", "content": "{rendered_system_prompt}" },
    { "role": "user",   "content": "{text}" }
  ],
  "temperature": {engine.temperature ?? 0.1},
  "stream": false
  // 当 engine.maxTokens 有值时附加 "max_tokens": {engine.maxTokens}
}
```

> `temperature` / `maxTokens` 为引擎可选字段(仅 OpenAI 兼容):`temperature` 缺省 0.1;
> `maxTokens` 缺省不发(由服务端默认)。设置页在「模型 / System Prompt」下方提供这两项编辑。

`rendered_system_prompt`:将 `systemPrompt` 中 `{sourceLang}` / `{targetLang}` 替换为实际语言名(使用 BCP-47 对应的英文全名,如 `Chinese Simplified`)。

默认 system prompt(当 `engine.systemPrompt` 为空时):

```
You are a professional translator.
Translate the following text from {sourceLang} to {targetLang}.
Output only the translation, no explanation, no extra content.
```

响应取 `choices[0].message.content`。

### 3.2 批量翻译(F3 使用)

`supports_batch` 返回 `true`。

将所有文本段拼为 JSON 数组形式的 user 消息:

```
Translate each item in the JSON array below from {sourceLang} to {targetLang}.
Return ONLY a JSON array of translated strings, same length, same order.

["原文1","原文2","原文3"]
```

响应解析:用 `serde_json::from_str::<Vec<String>>` 解析。
- 若解析失败 → fallback:逐条串行调用单条翻译(同一引擎)。
- 若长度不匹配 → 同上 fallback。

### 3.3 语言检测

OpenAI 引擎不做客户端语言检测;在 system prompt 里写 "auto-detect source language" 并要求模型在响应前加 `[LANG:xx-XX]` 前缀(可选特性,M3 后评估稳定性后决定是否启用)。

## 4. Google Translate 实现

文件:`core/translator/google.rs`

- 接口:Google Cloud Translation API v2(Basic)。
- 端点:`https://translation.googleapis.com/language/translate/v2`
- 参数:`q`(可多值)、`source`、`target`、`key`。
- 支持批量:一次请求传多个 `q`,返回 `data.translations[]`。
- 语言检测:响应中含 `detectedSourceLanguage`。
- 认证:`apiKey` 必填。仓库和发行二进制不携带共享 Key;缺少 Key 的 Google 引擎不会进入运行态路由。

## 5. DeepL 实现

文件:`core/translator/deepl.rs`

- 接口:DeepL API Free / Pro(`https://api-free.deepl.com/v2/translate` 或 `https://api.deepl.com/v2/translate`)。
- 认证:HTTP Header `Authorization: DeepL-Auth-Key {apiKey}`。
- 批量:传多个 `text` 字段。
- 语言代码映射:DeepL 用 `ZH`(不是 `zh-CN`)需在请求前转换。

## 6. 有道翻译实现

文件:`core/translator/youdao.rs`

- 接口:有道智云翻译 API。
- 认证:signType=v3 的 **SHA256** 摘要 `sha256(appKey + truncate(q) + salt + curtime + appSecret)`,需 `appKey` + `appSecret`(配置中 `apiKey` 存 `appKey:appSecret` 形式)。
  - `truncate(q)`:`len(q)<=20` 取原文,否则 `前10 + len + 后10`(按 Unicode 字符计)。
  - 注:早期 spec 写「HMAC-SHA256」,实际接口为上述 SHA256 摘要,已按真实接口实现。
- 批量:不支持,`supports_batch` 返回 `false`(引擎内部逐条串行)。

## 6.3 百度翻译实现

文件:`core/translator/baidu.rs`

- 接口:百度翻译开放平台「通用翻译 API」,`POST https://fanyi-api.baidu.com/api/trans/vip/translate`(表单)。参见 <https://fanyi-api.baidu.com/doc/21>。
- 参数:`q`、`from`、`to`、`appid`、`salt`、`sign`。
- 认证:**MD5** 签名 `sign = md5(appid + q + salt + 密钥)`(32 位小写十六进制,`q` 用未编码原文),需 `appid` + `密钥`(配置中 `apiKey` 存 `appid:secret` 形式,与有道同构)。
- 批量:不支持,`supports_batch` 返回 `false`(逐条串行);为保「译文条数 == 原文条数」不变量,不用 `\n` 拼接批量。
- 语言代码:百度用一批非标准码,需映射 —— 日 `jp`、韩 `kor`、法 `fra`、西 `spa`、阿 `ara`、繁体 `cht`。
- 百度与有道响应中的检测语言统一转换为 BCP-47 后再传给前端,保证后续重译可复用。当前映射未支持的语言明确返回 `unsupported language`,不静默转成英文。
- 错误:成功码为无 `error_code` 或 `52000`;其余非空 `error_code`(如 `54001` 签名错、`54003` 频率受限)→ `TranslateFailed("baidu_{code}")`。

## 6.1 微软 Azure Translator 实现

文件:`core/translator/microsoft.rs`

- 接口:Azure Translator Text API v3.0,`POST {host}/translate?api-version=3.0&to={to}[&from={from}]`,默认 host `https://api.cognitive.microsofttranslator.com`。
- 认证:Header `Ocp-Apim-Subscription-Key: {apiKey}` +(区域资源必填)`Ocp-Apim-Subscription-Region: {region}`。`region` 为 `TranslatorEngine` 新增字段(仅本引擎用)。
- 批量:支持,body 为 `[{"Text":"..."}, ...]`;响应 `[{translations:[{text}], detectedLanguage:{language}}]`。
- 语言代码:`zh-CN`→`zh-Hans`、`zh-Hant`→`zh-Hant`,其余取主子标签。

## 6.2 苹果系统翻译实现

文件:`core/translator/apple.rs`

- macOS 15+ 使用公开 Translation 框架。先用 `LanguageAvailability.status(from:to:)` 检查语言包;缺包时返回 `apple_languages_required` 安装指引,不创建会一直等待下载授权的翻译窗口。macOS 26+ 已安装语言包时使用 `TranslationSession(installedSource:target:)`;macOS 15 的已安装语言对使用 SwiftUI `translationTask` 宿主。
- `native/AppleTranslate.swift` 在构建时编译对应 CPU 架构 helper,嵌入 Rust 二进制。用户专属临时目录中 0700 helper 用 stdin/stdout JSON 桥接,不通过命令行传文本;取消/超时终止子进程。
- 批量结果按输入顺序返回;未知源语言用 NaturalLanguage 检测;同语言直接返回原文。
- Windows 和 macOS 15 以下不创建此运行态引擎,设置页说明限制并禁用启用/设默认;其他翻译引擎正常可用。
- 语言包通过“系统设置 → 通用 → 语言与地区 → 翻译语言”安装,设置页提供打开入口,需同时下载源语言和目标语言;连接测试源语言固定为英语。安装时间不计入请求超时,`timeoutMs`(1000~600000ms)只限制检查/翻译请求。测试失败消息完整换行显示,缺包和超时提供中英提示。
- API 依据:[Apple TranslationSession](https://developer.apple.com/documentation/translation/translationsession)、[Apple 语言包安装说明](https://support.apple.com/zh-cn/guide/mac-help/mchldd8b3c15/mac)。2026-10-15 本机 macOS 27.0.1 安装英/中语言包后跨语言单条和批量实测成功;macOS 15 宿主仍待单独验收。

## 7. HTTP Client(应用内代理已移除)

> 原「代理支持」已移除:不再提供应用内代理配置(`ProxyConfig` / `useProxy` 均已删除)。**编号 §7 保留不复用**,与 spec 08 的处理方式一致。

所有引擎共享管理器生命周期内的一个 `reqwest::Client`,以 `Lenz/{应用版本}` 作为 User-Agent,默认 30s,单次请求以引擎 `timeoutMs` 覆盖;修改目标语言不重建引擎或连接池。不做应用内代理配置,沿用 reqwest 默认行为(尊重系统环境变量 `HTTP(S)_PROXY` / `NO_PROXY`)。构建失败回退默认 client。

```rust
fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
        .user_agent(concat!("Lenz/", env!("CARGO_PKG_VERSION")))
        .timeout(Duration::from_secs(30))
        .build()
        .unwrap_or_else(|_| reqwest::Client::new())
}
```

如需代理访问云端引擎,由用户在系统/环境层配置。

## 8. Tauri Commands

```rust
#[tauri::command]
async fn translate_text(
    mgr: State<'_, TranslatorManager>,
    text: String,
    source_lang: Option<String>,
    target_lang: String,
    engine_id: Option<String>,
) -> Result<TranslateResponse, LenzError>;

#[tauri::command]
async fn translate_batch(
    mgr: State<'_, TranslatorManager>,
    texts: Vec<String>,
    source_lang: Option<String>,
    target_lang: String,
    engine_id: Option<String>,
) -> Result<TranslateResponse, LenzError>;

#[tauri::command]
async fn test_engine(
    engine_cfg: TranslatorEngine,
) -> Result<TestEngineResult, LenzError>;
```

`test_engine`:发送 "Hello" → 翻译为中文,返回延迟与结果,用于设置页引擎测试按钮。

## 9. 错误处理

| 场景 | LenzError |
|---|---|
| 网络超时 | `Timeout("response timed out")` |
| HTTP 4xx(API Key 错误) | `TranslateFailed("auth")` |
| HTTP 5xx | `TranslateFailed("server_error")` |
| 响应解析失败 | `TranslateFailed("parse_error")` |
| 批量 fallback 后仍失败 | 返回部分成功结果 + `TranslateFailed` |
| 引擎 id 不存在 | `Config("engine_not_found")` |

## 10. 验收标准

- [ ] LM Studio(本地)翻译一段英文 → 中文,端到端延迟 ≤ LLM 本身耗时 + 50ms。
- [ ] 批量翻译 10 条文本:OpenAI 兼容单次请求返回,Google/DeepL 也单次完成。
- [ ] 测试按钮能在 3s 内返回成功/失败结果。
- [ ] 引擎配置热更新后,下一次翻译使用新配置(无需重启)。

## 11. 开放问题

- [ ] OpenAI 批量翻译的 JSON 格式是否稳定——需在 M3 对多个模型做回归测试,决定是否加解析兜底。
- [ ] 流式输出(streaming):为 P2 预留,Translator trait 可将来扩展 `translate_stream` 方法。
- [x] 苹果系统翻译原生桥接:公开 `Translation` 框架、系统语言包预检查及安装入口,Windows 无此引擎;实现见 §6.2,macOS 15 宿主仍需实机验收。

## 12. 内置默认引擎预设

首次启动 / 重置时,`Config::default()` 预置以下引擎(`schema::default_engines()`);
id 用稳定 slug(`builtin-*`),`apiKey` / `region` 留空待用户填写。schema v1→v2 迁移在
`translator.engines` 为空时注入同一套预设;v2→v3 追加 `builtin-baidu`;v3→v4 移除
共享 Google Key 设计,关闭所有缺少凭据或尚未实现的内置引擎。首次启动默认引擎 id 为空,
用户启用首个可用引擎时再写入。

| id | 名称 | kind | 默认开启 | apiUrl | model |
|---|---|---|---|---|---|
| builtin-youdao | 有道翻译 | youdao | ❌(需 appKey:appSecret) | (默认端点) | — |
| builtin-apple | 苹果翻译 | apple | ❌(需 macOS 15+ 及语言包) | — | — |
| builtin-google | Google 翻译 | google | ❌(需 Key) | (默认端点) | — |
| builtin-zhipu | 智谱 GLM | openai | ❌(需 Key) | `open.bigmodel.cn/api/paas/v4` | glm-4-flash |
| builtin-siliconflow | 硅基流动 | openai | ❌(需 Key) | `api.siliconflow.cn/v1` | Qwen/Qwen2.5-7B-Instruct |
| builtin-baidu | 百度翻译 | baidu | ❌(需 appid:密钥) | (默认端点) | — |
| builtin-microsoft | 微软翻译 | microsoft | ❌ | (默认端点) | — |
| builtin-ollama | Ollama | openai | ❌ | `localhost:11434/v1` | qwen2.5 |
| builtin-lmstudio | LM Studio | openai | ❌ | `localhost:1234/v1` | — |

> OpenAI 兼容端点组装(`chat_endpoint`)容忍三种形态:已含 `/chat/completions` 原样;
> 已含版本段(`/v1`、`/api/paas/v4`)只补 `/chat/completions`;裸 host 补 `/v1/chat/completions`。
