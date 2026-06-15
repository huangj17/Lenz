/**
 * i18next 初始化。
 *
 * 语言来源:
 *  - 默认按浏览器/系统语言(navigator.language)前缀匹配;
 *  - 支持显式 "system" | "zh-CN" | "en-US"(对应 config.app.locale)。
 *
 * 所有 UI 文字通过 t("key") 引用,禁止硬编码中英文(spec 09 §6.4)。
 */
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import LanguageDetector from "i18next-browser-languagedetector";
import type { LocaleSetting } from "./types";
import zhCN from "../locales/zh-CN.json";
import enUS from "../locales/en-US.json";

export const SUPPORTED_LOCALES = ["zh-CN", "en-US"] as const;
export type SupportedLocale = (typeof SUPPORTED_LOCALES)[number];

export const DEFAULT_LOCALE: SupportedLocale = "en-US";

export const resources = {
  "zh-CN": { translation: zhCN },
  "en-US": { translation: enUS },
} as const;

/**
 * 把 BCP-47 语言标签按前缀匹配到受支持的 locale。
 * zh / zh-* → zh-CN;其余回退到默认。
 */
export function resolveLocale(tag: string | undefined): SupportedLocale {
  if (!tag) return DEFAULT_LOCALE;
  const lower = tag.toLowerCase();
  if (lower.startsWith("zh")) return "zh-CN";
  if (lower.startsWith("en")) return "en-US";
  return DEFAULT_LOCALE;
}

/** 把 config.app.locale 设置解析为具体 locale("system" → 跟随 navigator) */
export function resolveLocaleSetting(setting: LocaleSetting): SupportedLocale {
  if (setting === "system") {
    return resolveLocale(navigator.language);
  }
  return setting;
}

void i18n
  .use(LanguageDetector)
  .use(initReactI18next)
  .init({
    resources,
    fallbackLng: DEFAULT_LOCALE,
    supportedLngs: [...SUPPORTED_LOCALES],
    load: "currentOnly",
    interpolation: { escapeValue: false },
    detection: {
      // navigator 前缀匹配;不持久化(语言以 config 为准)
      order: ["navigator"],
      caches: [],
    },
  });

/** 切换语言(由设置页 / config 监听调用) */
export function applyLocale(setting: LocaleSetting): void {
  void i18n.changeLanguage(resolveLocaleSetting(setting));
}

export default i18n;
