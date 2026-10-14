import { useEffect } from "react";
import { MainWindow } from "@/windows/main/MainWindow";
import { PopupWindow } from "@/windows/popup/PopupWindow";
import { SelectionWindow } from "@/windows/selection/SelectionWindow";
import { OverlayWindow } from "@/windows/overlay/OverlayWindow";
import { IndicatorWindow } from "@/windows/indicator/IndicatorWindow";
import { useConfigSync } from "@/hooks/useConfigSync";
import { useSettingsShortcut } from "@/hooks/useSettingsShortcut";
import { useConfigStore } from "@/stores/config";

export type WindowKind = "main" | "popup" | "selection" | "overlay" | "indicator";

/** 从 URL 查询参数解析当前窗口类型,默认 main */
export function resolveWindowKind(search: string = window.location.search): WindowKind {
  const value = new URLSearchParams(search).get("window");
  switch (value) {
    case "popup":
    case "selection":
    case "overlay":
    case "indicator":
      return value;
    default:
      return "main";
  }
}

function renderWindow(kind: WindowKind) {
  switch (kind) {
    case "popup":
      return <PopupWindow />;
    case "selection":
      return <SelectionWindow />;
    case "overlay":
      return <OverlayWindow />;
    case "indicator":
      return <IndicatorWindow />;
    case "main":
    default:
      return <MainWindow />;
  }
}

/**
 * 应用根:按 ?window= 路由到对应窗口根组件,并统一同步主题/语言。
 *
 * 非 main 窗口也需要 config 来确定主题/语言,故同样加载一次(容错)。
 */
export function App({ kind }: { kind: WindowKind }) {
  useConfigSync();
  useSettingsShortcut();
  const load = useConfigStore((s) => s.load);
  const loaded = useConfigStore((s) => s.loaded);

  // main 窗口在 MainWindow 内已 load;其它窗口在此补一次
  useEffect(() => {
    if (kind !== "main" && !loaded) void load();
    document.body.dataset.window = kind;
  }, [kind, loaded, load]);

  return renderWindow(kind);
}

export default App;
