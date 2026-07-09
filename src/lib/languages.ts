/**
 * 常用翻译语言（BCP-47）。弹框语言下拉切换复用;显示名一律经 `Intl.DisplayNames`
 * 按当前 UI 语言本地化,故此处只维护 code 列表。顺序 = 下拉展示顺序(中英日韩优先)。
 */
export const TRANSLATE_LANGUAGES = [
  "zh-CN",
  "zh-TW",
  "en",
  "ja",
  "ko",
  "fr",
  "de",
  "es",
  "ru",
  "pt",
  "it",
  "ar",
  "hi",
  "th",
  "vi",
  "id",
  "nl",
  "pl",
  "tr",
] as const;
