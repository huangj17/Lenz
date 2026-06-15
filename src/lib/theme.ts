/**
 * 主题应用:把 config.app.theme(system|light|dark)落到 <html>.dark 类。
 *
 * - "system":跟随 prefers-color-scheme,并监听其变化;
 * - "light" / "dark":强制。
 *
 * 参考 spec 09 §6.1。Rust 侧系统主题变化会 emit config 事件,
 * "system" 模式下本模块的 media query 监听也会就地响应。
 */
import type { ThemeSetting } from "./types";

const root = () => document.documentElement;
const media = () => window.matchMedia("(prefers-color-scheme: dark)");

function setDarkClass(isDark: boolean): void {
  root().classList.toggle("dark", isDark);
}

let cleanupSystemListener: (() => void) | null = null;

/**
 * 应用主题设置;返回清理函数(解绑 system 监听)。
 * 重复调用会先清理上一次的监听。
 */
export function applyTheme(theme: ThemeSetting): () => void {
  cleanupSystemListener?.();
  cleanupSystemListener = null;

  if (theme === "system") {
    const mq = media();
    const onChange = (e: MediaQueryListEvent) => setDarkClass(e.matches);
    setDarkClass(mq.matches);
    mq.addEventListener("change", onChange);
    cleanupSystemListener = () => mq.removeEventListener("change", onChange);
  } else {
    setDarkClass(theme === "dark");
  }

  return () => {
    cleanupSystemListener?.();
    cleanupSystemListener = null;
  };
}
