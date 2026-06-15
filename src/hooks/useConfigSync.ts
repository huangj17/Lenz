import { useEffect } from "react";
import { useConfigStore } from "@/stores/config";
import { applyTheme } from "@/lib/theme";
import { applyLocale } from "@/lib/i18n";
import { configApi, Events, listenEvent } from "@/lib/ipc";

/**
 * 把 config 的主题 / 语言落到 DOM 与 i18n,并订阅后端 config://changed
 * 以便任一窗口修改配置后全部窗口即时刷新。
 *
 * 在每个窗口根挂载一次。M1 后端可能不发事件,监听失败被 ipc 容错吞掉。
 */
export function useConfigSync() {
  const theme = useConfigStore((s) => s.config.app.theme);
  const locale = useConfigStore((s) => s.config.app.locale);
  const setConfig = useConfigStore((s) => s.setConfig);

  // 主题 → <html>.dark(system 模式内部还会监听 prefers-color-scheme)
  useEffect(() => applyTheme(theme), [theme]);

  // 语言 → i18next
  useEffect(() => applyLocale(locale), [locale]);

  // 订阅后端配置变更:重新拉取完整 config(keypath 用于将来做细粒度刷新)
  useEffect(() => {
    let unlisten: (() => void) | undefined;
    let active = true;

    listenEvent(Events.ConfigChanged, () => {
      void configApi
        .get()
        .then((cfg) => setConfig(cfg))
        .catch(() => {
          /* ipc 已分发错误 */
        });
    })
      .then((fn) => {
        if (active) unlisten = fn;
        else fn();
      })
      .catch(() => {
        /* 无 Tauri 环境 / 命令未注册:容错忽略 */
      });

    return () => {
      active = false;
      unlisten?.();
    };
  }, [setConfig]);
}
