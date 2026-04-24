# Spec 04 · 翻译引擎

> 覆盖 PRD §2.2 F4 与 §3.4(翻译引擎抽象)。定义 Translator trait、各引擎实现、多引擎管理与代理支持。

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
    OpenAICompat,
    Google,
    DeepL,
    Youdao,
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
    http_client: reqwest::Client,       // 共享,支持代理
}

impl TranslatorManager {
    pub fn reload(&self, configs: Vec<TranslatorEngine>, proxy: &ProxyConfig);
    pub fn get(&self, id: Option<&str>) -> Option<Arc<dyn Translator>>;
    pub async fn translate(&self, id: Option<&str>, req: TranslateRequest)
        -> Result<TranslateResponse, LenzError>;
}
```

- 配置变更(`config://changed` 含 `translator.*`)时调 `reload`,不重启进程。
- `http_client` 按代理配置重建(见 §6)。

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
  "temperature": 0.1,
  "stream": false
}
```

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

## 5. DeepL 实现

文件:`core/translator/deepl.rs`

- 接口:DeepL API Free / Pro(`https://api-free.deepl.com/v2/translate` 或 `https://api.deepl.com/v2/translate`)。
- 认证:HTTP Header `Authorization: DeepL-Auth-Key {apiKey}`。
- 批量:传多个 `text` 字段。
- 语言代码映射:DeepL 用 `ZH`(不是 `zh-CN`)需在请求前转换。

## 6. 有道翻译实现

文件:`core/translator/youdao.rs`

- 接口:有道智云翻译 API。
- 认证:HMAC-SHA256 签名,需 `appKey` + `appSecret`(配置中 `apiKey` 存 `appKey:appSecret` 形式)。
- 批量:不支持,`supports_batch` 返回 `false`(manager 层自动逐条调用)。

## 7. 代理支持

`TranslatorManager` 内的 `reqwest::Client` 按 `ProxyConfig` 构建:

```rust
fn build_http_client(proxy: &ProxyConfig) -> reqwest::Client {
    let mut builder = reqwest::Client::builder()
        .timeout(Duration::from_secs(30));
    if proxy.enabled {
        let proxy_url = format!("{}://{}:{}", proxy.scheme, proxy.host, proxy.port);
        let mut p = reqwest::Proxy::all(&proxy_url).unwrap();
        if let (Some(u), Some(pw)) = (&proxy.username, &proxy.password) {
            p = p.basic_auth(u, pw);
        }
        builder = builder.proxy(p);
        if proxy.bypass_local {
            // reqwest 自动 bypass 127.0.0.1,无需额外配置
        }
    }
    builder.build().unwrap()
}
```

本地 LLM(LM Studio/Ollama)引擎:`engine.useProxy = false` → 该引擎的请求用 `reqwest::Client::new()`(无代理)。

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
    proxy: ProxyConfig,
) -> Result<TestEngineResult, LenzError>;
```

`test_engine`:发送 "Hello" → 翻译为中文,返回延迟与结果,用于设置页引擎测试按钮。

## 9. 错误处理

| 场景 | LenzError |
|---|---|
| 网络超时 | `Network("timeout")` |
| HTTP 4xx(API Key 错误) | `TranslateFailed("auth")` |
| HTTP 5xx | `TranslateFailed("server_error")` |
| 响应解析失败 | `TranslateFailed("parse_error")` |
| 批量 fallback 后仍失败 | 返回部分成功结果 + `TranslateFailed` |
| 引擎 id 不存在 | `Config("engine_not_found")` |

## 10. 验收标准

- [ ] LM Studio(本地)翻译一段英文 → 中文,端到端延迟 ≤ LLM 本身耗时 + 50ms。
- [ ] 批量翻译 10 条文本:OpenAI 兼容单次请求返回,Google/DeepL 也单次完成。
- [ ] 测试按钮能在 3s 内返回成功/失败结果。
- [ ] 代理配置后 `curl` 同地址能通,翻译也通。
- [ ] 引擎配置热更新后,下一次翻译使用新配置(无需重启)。

## 11. 开放问题

- [ ] OpenAI 批量翻译的 JSON 格式是否稳定——需在 M3 对多个模型做回归测试,决定是否加解析兜底。
- [ ] 流式输出(streaming):为 P2 预留,Translator trait 可将来扩展 `translate_stream` 方法。
