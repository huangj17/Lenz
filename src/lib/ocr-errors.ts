const tencentErrors = new Set([
  "ocr_tencent_credentials",
  "ocr_tencent_clock",
  "ocr_tencent_empty_image",
  "ocr_tencent_image_too_large",
  "ocr_tencent_signature_expired",
  "ocr_tencent_not_enabled",
  "ocr_tencent_quota",
  "ocr_tencent_auth",
  "ocr_tencent_rate_limit",
  "ocr_tencent_failed",
]);

/** 样例测试返回标识;截图事件携带 Display 文本;覆盖层携带序列化 message。 */
export function tencentOcrErrorKey(message: string): string | null {
  const code = message.replace(/^(?:ocr failed|config error): /, "");
  return tencentErrors.has(code) ? `ocr.errors.${code}` : null;
}
