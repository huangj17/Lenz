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
/// v3:新增百度翻译预设(`builtin-baidu`,默认关闭)+ 谷歌翻译预设。
/// v4:移除仓库内置 Google Key;所有需凭据或尚未实现的引擎默认关闭。
/// v5:系统 OCR 识别偏好;v6:OCR 服务列表、默认服务及第三方凭据。
/// v7:按服务记忆弹窗折叠与跳过翻译策略。
pub const CURRENT_VERSION: u32 = 7;

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
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

/// OCR 服务与系统识别偏好。v6 将单一系统引擎迁移为服务列表。
#[derive(Debug, Clone, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrConfig {
    pub default_service_id: String,
    pub services: Vec<OcrService>,
    /// 偏好识别语言(BCP-47)。
    pub preferred_languages: Vec<String>,
    /// 最低置信度过滤阈值,0~1。
    pub min_confidence: f32,
    #[serde(default)]
    pub fast_mode: bool,
    #[serde(default = "default_true")]
    pub auto_detect_language: bool,
}

fn default_true() -> bool {
    true
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub enum OcrServiceKind {
    System,
    Baidu,
    /// 支持图片输入的 OpenAI Chat Completions 兼容接口,仅用于截图翻译。
    Vision,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase")]
pub struct OcrService {
    pub id: String,
    pub name: String,
    pub kind: OcrServiceKind,
    pub api_url: String,
    pub api_key: String,
    /// 百度 OCR 的 Secret Key,与百度翻译凭据不同。
    pub secret_key: String,
    pub model: String,
    pub timeout_ms: u32,
    /// 百度高精度含位置版;false 使用标准含位置版。
    pub baidu_accurate: bool,
    pub baidu_language: String,
}

pub const SYSTEM_OCR_ID: &str = "system";

pub fn default_ocr_services() -> Vec<OcrService> {
    vec![OcrService {
        id: SYSTEM_OCR_ID.into(),
        name: "System OCR".into(),
        kind: OcrServiceKind::System,
        api_url: String::new(),
        api_key: String::new(),
        secret_key: String::new(),
        model: String::new(),
        timeout_ms: 30_000,
        baidu_accurate: true,
        baidu_language: "CHN_ENG".into(),
    }]
}

pub fn validate_ocr_config(config: &OcrConfig) -> Result<(), crate::error::LenzError> {
    use std::collections::HashSet;
    let mut ids = HashSet::new();
    for service in &config.services {
        if service.id.trim().is_empty()
            || !ids.insert(service.id.as_str())
            || service.name.trim().is_empty()
            || !(1_000..=300_000).contains(&service.timeout_ms)
            || (service.id == SYSTEM_OCR_ID) != (service.kind == OcrServiceKind::System)
        {
            return Err(crate::error::LenzError::Config(
                "invalid_ocr_services".into(),
            ));
        }
    }
    if !ids.contains(SYSTEM_OCR_ID) || !ids.contains(config.default_service_id.as_str()) {
        return Err(crate::error::LenzError::Config(
            "invalid_ocr_default".into(),
        ));
    }
    Ok(())
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
    /// 百度翻译开放平台(通用翻译 API,MD5 签名;需 appid + 密钥)。
    Baidu,
    /// 微软 Azure Translator(需 Key + region)。
    Microsoft,
    /// 苹果系统翻译(macOS 原生框架;当前为占位,见 spec 04 §11)。
    Apple,
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
    /// 弹框逻辑宽度(用户可拖拽右缘调节并记忆),默认 360;取值夹 [300, 720]。
    #[serde(default = "default_popup_width")]
    pub width: u32,
    /// 按翻译服务 id 保存的弹窗折叠策略。
    #[serde(default)]
    pub services: std::collections::BTreeMap<String, PopupServiceUiConfig>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, Type)]
#[serde(rename_all = "camelCase", default)]
pub struct PopupServiceUiConfig {
    pub collapsed: bool,
    pub remember_collapse: bool,
    pub skip_when_collapsed: bool,
}

impl PopupServiceUiConfig {
    pub fn skip_initial_translation(&self) -> bool {
        self.remember_collapse && self.collapsed && self.skip_when_collapsed
    }
}

fn default_popup_width() -> u32 {
    360
}

/// 内置默认翻译引擎预设(spec 04 §12)。
///
/// 所有需凭据、外部服务或尚未实现的引擎默认关闭。用户填好配置并测试成功后再开启;
/// 这避免首次启动把文本发给任意第三方,也避免默认路由到必然鉴权失败的引擎。
/// id 用稳定 slug 便于跨安装引用。
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
            region: None,
            max_tokens: None,
            temperature: None,
        }
    }

    vec![
        e(
            "builtin-youdao",
            "有道翻译",
            EngineKind::Youdao,
            false,
            "",
            None,
            15_000,
        ),
        e(
            "builtin-apple",
            "苹果翻译",
            EngineKind::Apple,
            false,
            "",
            None,
            15_000,
        ),
        e(
            "builtin-google",
            "Google 翻译",
            EngineKind::Google,
            false,
            "",
            None,
            15_000,
        ),
        e(
            "builtin-zhipu",
            "智谱 GLM",
            EngineKind::Openai,
            false,
            "https://open.bigmodel.cn/api/paas/v4",
            Some("glm-4-flash"),
            30_000,
        ),
        e(
            "builtin-siliconflow",
            "硅基流动",
            EngineKind::Openai,
            false,
            "https://api.siliconflow.cn/v1",
            Some("Qwen/Qwen2.5-7B-Instruct"),
            30_000,
        ),
        e(
            "builtin-baidu",
            "百度翻译",
            EngineKind::Baidu,
            false,
            "",
            None,
            15_000,
        ),
        e(
            "builtin-microsoft",
            "微软翻译",
            EngineKind::Microsoft,
            false,
            "",
            None,
            15_000,
        ),
        e(
            "builtin-ollama",
            "Ollama",
            EngineKind::Openai,
            false,
            "http://localhost:11434/v1",
            Some("qwen2.5"),
            60_000,
        ),
        e(
            "builtin-lmstudio",
            "LM Studio",
            EngineKind::Openai,
            false,
            "http://localhost:1234/v1",
            None,
            60_000,
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
                // 菜单栏驻留工具:默认启动不弹主窗口,仅驻留托盘(隐私友好)。
                // 同时规避 macOS 上「关闭 popup/选区小窗时,常驻可见的主窗口被顶到前台」。
                open_on_launch: false,
            },
            shortcuts: ShortcutConfig {
                selection: Some("Alt+D".into()),
                screenshot: Some("Alt+S".into()),
                overlay: Some("Alt+F".into()),
                open_main: None,
            },
            selection: SelectionConfig {
                // 划词监听(划词指示图标)默认关闭,opt-in:用户在托盘勾选「划词监听」
                // 后开启(需辅助功能权限)。Alt+D 快捷键划词不受影响,始终可用。
                enabled: false,
                // 避免误触;用户按需开启。
                auto_detect: false,
                popup_delay_ms: 300,
            },
            ocr: OcrConfig {
                default_service_id: SYSTEM_OCR_ID.into(),
                services: default_ocr_services(),
                preferred_languages: vec!["en-US".into(), "zh-Hans".into()],
                min_confidence: 0.3,
                fast_mode: false,
                auto_detect_language: true,
            },
            translator: TranslatorConfig {
                // 首次启动不隐式选择第三方服务;用户启用引擎时再设置默认项。
                default_engine_id: String::new(),
                target_lang: "zh-CN".into(),
                engines: default_engines(),
            },
            ui: UiConfig {
                popup: PopupUiConfig {
                    font_size: 14,
                    show_original: true,
                    width: 360,
                    services: Default::default(),
                },
            },
        }
    }
}
