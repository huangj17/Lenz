import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { clsx } from "clsx";
import { LensMark } from "@/components/LensMark";
import { Icon, type IconName } from "@/components/Icon";
import { useConfigStore } from "@/stores/config";
import { GeneralPane } from "./panes/GeneralPane";
import { ShortcutsPane } from "./panes/ShortcutsPane";
import { ServicesPane } from "./panes/ServicesPane";
import { AboutPane } from "./panes/AboutPane";

type TabId = "general" | "shortcuts" | "services" | "about";

const NAV: { id: TabId; icon: IconName }[] = [
  { id: "general", icon: "settings" },
  { id: "shortcuts", icon: "keyboard" },
  { id: "services", icon: "cpu" },
  { id: "about", icon: "info" },
];

/**
 * 设置主窗口 —— 对齐设计稿:200px vibrancy 侧边栏(红绿灯留白 + logo + 4 导航 +
 * 底部连接状态)+ 内容区(header 标题/搜索框 + 当前 pane)。
 *
 * macOS 用 titleBarStyle: Overlay,原生红绿灯浮于侧边栏左上;顶部 data-tauri-drag-region 可拖拽。
 */
export function MainWindow() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<TabId>("general");
  const load = useConfigStore((s) => s.load);
  const engines = useConfigStore((s) => s.config.translator.engines);
  const defaultId = useConfigStore((s) => s.config.translator.defaultEngineId);

  useEffect(() => {
    void load();
  }, [load]);

  const enabledEngines = engines.filter((e) => e.enabled);
  const def = enabledEngines.find((e) => e.id === defaultId) ?? enabledEngines[0];
  const endpoint = def ? def.apiUrl.replace(/^https?:\/\//, "") : null;

  return (
    <div className="grid h-screen grid-cols-[200px_1fr] bg-bg text-fg">
      {/* 侧边栏 */}
      <aside className="ln-vibrancy flex flex-col gap-0.5 border-r-[0.5px] border-line px-2.5 py-3">
        {/* macOS overlay 原生红绿灯留白 + 可拖拽:留足高度让 logo 与红绿灯拉开间距 */}
        <div data-tauri-drag-region className="h-7" />
        <div
          data-tauri-drag-region
          className="flex items-center gap-2 px-2 pb-2.5"
        >
          <LensMark size={22} />
          <span className="text-sm font-semibold tracking-tight">Lenz</span>
          <span className="ml-auto font-mono text-[10px] text-sub">0.1.0</span>
        </div>

        {NAV.map((n) => {
          const active = tab === n.id;
          return (
            <button
              key={n.id}
              type="button"
              onClick={() => setTab(n.id)}
              className={clsx(
                "flex cursor-pointer items-center gap-[9px] rounded-[7px] border-0 bg-transparent px-[9px] py-[7px] text-left text-[13px] font-medium",
                active ? "text-accent-ink" : "text-fg",
              )}
              style={active ? { background: "var(--tint)" } : undefined}
            >
              <Icon name={n.icon} size={15} />
              <span>{t(`nav.${n.id}`)}</span>
            </button>
          );
        })}

        {/* 底部连接状态 */}
        <div className="mt-auto border-t-[0.5px] border-line p-2 text-[11px] text-sub">
          <div className="flex items-center gap-1.5">
            <span
              className={clsx("size-1.5 rounded-full", def && "ln-glow")}
              style={{
                background: def
                  ? "var(--sage)"
                  : "color-mix(in srgb, var(--fg) 25%, transparent)",
              }}
            />
            <span>{def ? def.name : t("sidebar.noEngine")}</span>
          </div>
          {endpoint && (
            <div className="mt-0.5 truncate font-mono text-[10px] text-sub">
              {endpoint}
            </div>
          )}
        </div>
      </aside>

      {/* 内容 */}
      <main className="flex min-h-0 flex-col">
        <header
          data-tauri-drag-region
          className="ln-header flex items-center justify-between border-b-[0.5px] border-line px-6 py-[14px]"
        >
          <div data-tauri-drag-region className="text-[15px] font-semibold">
            {t(`nav.${tab}`)}
          </div>
          <div className="flex items-center gap-1.5 rounded-md border-[0.5px] border-line bg-field py-1 pr-2.5 pl-2 text-xs text-sub">
            <Icon name="search" size={13} />
            <span>{t("header.search")}</span>
          </div>
        </header>

        {tab === "services" ? (
          <div className="min-h-0 flex-1 overflow-hidden px-6 pt-[22px] pb-6">
            <ServicesPane />
          </div>
        ) : (
          <div className="min-h-0 flex-1 overflow-y-auto px-6 pt-[22px] pb-6">
            {tab === "general" && <GeneralPane />}
            {tab === "shortcuts" && <ShortcutsPane />}
            {tab === "about" && <AboutPane />}
          </div>
        )}
      </main>
    </div>
  );
}

export default MainWindow;
