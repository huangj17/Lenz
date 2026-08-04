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
    Apple,          // 苹果系统翻译(macOS 原生,当前占位,见 §11)
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
- 错误:成功码为无 `error_code` 或 `52000`;其余非空 `error_code`(如 `54001` 签名错、`54003` 频率受限)→ `TranslateFailed("baidu_{code}")`。

## 6.1 微软 Azure Translator 实现

文件:`core/translator/microsoft.rs`

- 接口:Azure Translator Text API v3.0,`POST {host}/translate?api-version=3.0&to={to}[&from={from}]`,默认 host `https://api.cognitive.microsofttranslator.com`。
- 认证:Header `Ocp-Apim-Subscription-Key: {apiKey}` +(区域资源必填)`Ocp-Apim-Subscription-Region: {region}`。`region` 为 `TranslatorEngine` 新增字段(仅本引擎用)。
- 批量:支持,body 为 `[{"Text":"..."}, ...]`;响应 `[{translations:[{text}], detectedLanguage:{language}}]`。
- 语言代码:`zh-CN`→`zh-Hans`、`zh-Hant`→`zh-Hant`,其余取主子标签。

## 6.2 苹果系统翻译实现(占位)

文件:`core/translator/apple.rs`

- macOS `Translation` 框架无干净的无界面公开 API(`TranslationSession` 仅能经 SwiftUI 获取),当前为**占位**:可被配置 / 选中,`translate` 返回 `TranslateFailed("apple_not_implemented")`。
- 后续接原生桥接(隐藏 SwiftUI 宿主 + 按需下载语言包),见 §11。

## 7. HTTP Client(应用内代理已移除)

> 原「代理支持」已移除:不再提供应用内代理配置(`ProxyConfig` / `useProxy` 均已删除)。**编号 §7 保留不复用**,与 spec 08 的处理方式一致。

所有引擎共享一个 `reqwest::Client`,仅设 30s 超时,不做任何应用内代理配置——沿用 reqwest 默认行为(尊重系统环境变量 `HTTP(S)_PROXY` / `NO_PROXY`)。构建失败回退默认 client。

```rust
fn build_http_client() -> reqwest::Client {
    reqwest::Client::builder()
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
- [ ] 引擎配置热更新后,下一次翻译使用新配置(无需重启)。

## 11. 开放问题

- [ ] OpenAI 批量翻译的 JSON 格式是否稳定——需在 M3 对多个模型做回归测试,决定是否加解析兜底。
- [ ] 流式输出(streaming):为 P2 预留,Translator trait 可将来扩展 `translate_stream` 方法。
- [ ] 苹果系统翻译原生桥接:macOS `Translation` 框架无头调用方案(隐藏 SwiftUI 宿主 / Shortcuts / 私有框架),含语言包下载与跨平台(Windows 无此引擎)处理。

## 12. 内置默认引擎预设

首次启动 / 重置时,`Config::default()` 预置以下引擎(`schema::default_engines()`);
id 用稳定 slug(`builtin-*`),`apiKey` / `region` 留空待用户填写。schema v1→v2 迁移在
`translator.engines` 为空时注入同一套预设;v2→v3 追加 `builtin-baidu`;v3→v4 移除
共享 Google Key 设计,关闭所有缺少凭据或尚未实现的内置引擎。首次启动默认引擎 id 为空,
用户启用首个可用引擎时再写入。

| id | 名称 | kind | 默认开启 | apiUrl | model |
|---|---|---|---|---|---|
| builtin-youdao | 有道翻译 | youdao | ❌(需 appKey:appSecret) | (默认端点) | — |
| builtin-apple | 苹果翻译 | apple | ❌(尚未实现) | — | — |
| builtin-google | Google 翻译 | google | ❌(需 Key) | (默认端点) | — |
| builtin-zhipu | 智谱 GLM | openai | ❌(需 Key) | `open.bigmodel.cn/api/paas/v4` | glm-4-flash |
| builtin-siliconflow | 硅基流动 | openai | ❌(需 Key) | `api.siliconflow.cn/v1` | Qwen/Qwen2.5-7B-Instruct |
| builtin-baidu | 百度翻译 | baidu | ❌(需 appid:密钥) | (默认端点) | — |
| builtin-microsoft | 微软翻译 | microsoft | ❌ | (默认端点) | — |
| builtin-ollama | Ollama | openai | ❌ | `localhost:11434/v1` | qwen2.5 |
| builtin-lmstudio | LM Studio | openai | ❌ | `localhost:1234/v1` | — |

> OpenAI 兼容端点组装(`chat_endpoint`)容忍三种形态:已含 `/chat/completions` 原样;
> 已含版本段(`/v1`、`/api/paas/v4`)只补 `/chat/completions`;裸 host 补 `/v1/chat/completions`。
