//! 配置 Schema(spec 01 §2)与默认值(spec 01 §3)。
//!
//! Rust 端用 serde derive 与前端 TypeScript 接口保持同构;线缆格式 camelCase。
//! 注意:历史记录功能已删除,Config 中没有 `history` 字段。

use serde::{Deserialize, Serialize};
use specta::Type;

/// 当前 schema 版本号,用于迁移(spec 01 §7)。
///
/// v2:内置默认翻译引擎预设(有道/苹果/智谱/硅基流动 默认开启,
/// 谷歌/微软/ollama/LM Studio 默认关闭)+ `TranslatorEngine.region` 字段。
pub const CURRENT_VERSION: u32 = 2;

/// 顶层配置。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct Config {
    /// schema 版本号,用于迁移。
    pub version: u32,
    pub app: AppConfig,
    pub shortcuts: ShortcutConfig,
    pub selection: SelectionConfig,
    pub ocr: OcrConfig,
    pub translator: TranslatorConfig,
    pub proxy: ProxyConfig,
    pub ui: UiConfig,
}

/// 应用级配置。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct AppConfig {
    pub locale: Locale,
    pub theme: Theme,
    pub auto_start: bool,
    pub check_update: bool,
    /// 启动时是否展示主窗口。
    pub open_on_launch: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "kebab-case")]
pub enum Locale {
    System,
    #[serde(rename = "zh-CN")]
    ZhCn,
    #[serde(rename = "en-US")]
    EnUs,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum Theme {
    System,
    Light,
    Dark,
}

/// 全局快捷键。`None` 表示禁用对应功能。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ShortcutConfig {
    /// 划词翻译快捷键,如 "Alt+D"。
    pub selection: Option<String>,
    /// 截图翻译快捷键,如 "Alt+S"。
    pub screenshot: Option<String>,
    /// 原图覆盖翻译快捷键,如 "Alt+F"。
    pub overlay: Option<String>,
    /// 打开主窗口快捷键。
    pub open_main: Option<String>,
}

/// 划词监听相关。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct SelectionConfig {
    /// 总开关。
    pub enabled: bool,
    /// 是否启用自动弹框(选中即译)。
    pub auto_detect: bool,
    /// 选中后延迟触发的去抖时长(毫秒)。
    pub popup_delay_ms: u32,
}

/// OCR 配置。MVP 仅支持系统引擎。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrConfig {
    pub engine: OcrEngineKind,
    /// 偏好识别语言(BCP-47)。
    pub preferred_languages: Vec<String>,
    /// 最低置信度过滤阈值,0~1。
    pub min_confidence: f32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OcrEngineKind {
    System,
}

/// 翻译引擎集合配置。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranslatorConfig {
    /// 默认引擎 id。
    pub default_engine_id: String,
    /// 默认目标语言,默认 "zh-CN"。
    pub target_lang: String,
    /// 用户配置的引擎列表。
    pub engines: Vec<TranslatorEngine>,
}

/// 单个翻译引擎(spec 01 §2)。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct TranslatorEngine {
    /// uuid。
    pub id: String,
    /// 用户自定义名称。
    pub name: String,
    pub kind: EngineKind,
    pub enabled: bool,
    pub api_url: String,
    /// 凭据,明文保存(MVP,见 spec 01 §6)。
    pub api_key: String,
    /// LLM 类必填的模型名。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    /// LLM 类的 system prompt,支持 `{sourceLang}` / `{targetLang}` 变量。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub system_prompt: Option<String>,
    /// 超时(毫秒),默认 15000。
    pub timeout_ms: u32,
    /// 是否走代理。本地 LLM 通常为 false。
    pub use_proxy: bool,
    /// Azure 翻译所需区域(仅 Microsoft 引擎用),如 "eastasia";其余引擎为 None。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub region: Option<String>,
    /// LLM 最大输出 token 数(仅 OpenAI 兼容);None = 不限,由服务端默认。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_tokens: Option<u32>,
    /// LLM 采样温度(仅 OpenAI 兼容);None = 引擎默认 0.1。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub temperature: Option<f32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum EngineKind {
    /// OpenAI 兼容(LM Studio / Ollama / 智谱 / 硅基流动 / DeepSeek …)。
    Openai,
    /// Google Cloud Translation v2(官方,需 Key)。
    Google,
    /// DeepL Free / Pro。
    Deepl,
    /// 有道智云(signType=v3)。
    Youdao,
    /// 微软 Azure Translator(需 Key + region)。
    Microsoft,
    /// 苹果系统翻译(macOS 原生框架;当前为占位,见 spec 04 §11)。
    Apple,
}

/// 代理配置。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct ProxyConfig {
    pub enabled: bool,
    pub scheme: ProxyScheme,
    pub host: String,
    pub port: u16,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub username: Option<String>,
    /// 明文保存(MVP)。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
    /// 默认 true:127.0.0.1 / localhost 绕过代理。
    pub bypass_local: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum ProxyScheme {
    Http,
    Https,
    Socks5,
}

/// UI 偏好。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct UiConfig {
    pub popup: PopupUiConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct PopupUiConfig {
    /// 字号,默认 14。
    pub font_size: u32,
    /// 是否展示原文。
    pub show_original: bool,
}

/// 内置默认翻译引擎预设(spec 04 §12)。
///
/// 默认开启:有道、苹果、智谱、硅基流动;默认关闭:谷歌、微软、ollama、LM Studio。
/// 凭据(`api_key` / `region`)留空待用户填写;id 用稳定 slug 便于跨安装引用。
pub fn default_engines() -> Vec<TranslatorEngine> {
    // 统一构造:api_key/region 留空、system_prompt 默认 None。
    #[allow(clippy::too_many_arguments)]
    fn e(
        id: &str,
        name: &str,
        kind: EngineKind,
        enabled: bool,
        api_url: &str,
        model: Option<&str>,
        timeout_ms: u32,
        use_proxy: bool,
    ) -> TranslatorEngine {
        TranslatorEngine {
            id: id.into(),
            name: name.into(),
            kind,
            enabled,
            api_url: api_url.into(),
            api_key: String::new(),
            model: model.map(Into::into),
            system_prompt: None,
            timeout_ms,
            use_proxy,
            region: None,
            max_tokens: None,
            temperature: None,
        }
    }

    vec![
        // —— 默认开启 ——
        e("builtin-youdao", "有道翻译", EngineKind::Youdao, true, "", None, 15_000, false),
        e("builtin-apple", "苹果翻译", EngineKind::Apple, true, "", None, 15_000, false),
        e(
            "builtin-zhipu",
            "智谱 GLM",
            EngineKind::Openai,
            true,
            "https://open.bigmodel.cn/api/paas/v4",
            Some("glm-4-flash"),
            30_000,
            false,
        ),
        e(
            "builtin-siliconflow",
            "硅基流动",
            EngineKind::Openai,
            true,
            "https://api.siliconflow.cn/v1",
            Some("Qwen/Qwen2.5-7B-Instruct"),
            30_000,
            false,
        ),
        // —— 默认关闭 ——
        e("builtin-google", "Google 翻译", EngineKind::Google, false, "", None, 15_000, true),
        e("builtin-microsoft", "微软翻译", EngineKind::Microsoft, false, "", None, 15_000, false),
        e(
            "builtin-ollama",
            "Ollama",
            EngineKind::Openai,
            false,
            "http://localhost:11434/v1",
            Some("qwen2.5"),
            60_000,
            false,
        ),
        e(
            "builtin-lmstudio",
            "LM Studio",
            EngineKind::Openai,
            false,
            "http://localhost:1234/v1",
            None,
            60_000,
            false,
        ),
    ]
}

impl Default for Config {
    /// 首次启动生成的默认配置(spec 01 §3)。
    ///
    /// 快捷键默认值参考 PRD §4.1 / spec 10 §1.2:划词 Alt+D、截图 Alt+S、
    /// 原图覆盖 Alt+F。
    fn default() -> Self {
        Config {
            version: CURRENT_VERSION,
            app: AppConfig {
                locale: Locale::System,
                theme: Theme::System,
                auto_start: false,
                check_update: true,
                open_on_launch: true,
            },
            shortcuts: ShortcutConfig {
                selection: Some("Alt+D".into()),
                screenshot: Some("Alt+S".into()),
                overlay: Some("Alt+F".into()),
                open_main: None,
            },
            selection: SelectionConfig {
                enabled: true,
                // 避免误触;用户按需开启。
                auto_detect: false,
                popup_delay_ms: 300,
            },
            ocr: OcrConfig {
                engine: OcrEngineKind::System,
                preferred_languages: vec!["en-US".into(), "zh-Hans".into()],
                min_confidence: 0.3,
            },
            translator: TranslatorConfig {
                // 默认路由到有道(用户列表里第一个默认开启项);用户可在设置页改默认。
                default_engine_id: "builtin-youdao".into(),
                target_lang: "zh-CN".into(),
                engines: default_engines(),
            },
            proxy: ProxyConfig {
                enabled: false,
                scheme: ProxyScheme::Http,
                host: String::new(),
                port: 7890,
                username: None,
                password: None,
                bypass_local: true,
            },
            ui: UiConfig {
                popup: PopupUiConfig {
                    font_size: 14,
                    show_original: true,
                },
            },
        }
    }
}
