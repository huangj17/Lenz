import { listen, onLenzError, systemApi, emit, Events } from "@/lib/ipc";
import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { clsx } from "clsx";
import { LensMark } from "@/components/LensMark";
import { Icon, type IconName } from "@/components/Icon";
import { useConfigStore } from "@/stores/config";
import { GeneralPane } from "./panes/GeneralPane";
import { ShortcutsPane } from "./panes/ShortcutsPane";
import { ServicesPane } from "./panes/ServicesPane";
import { useSystemStore } from "@/stores/system";
import type { UpdateInfo, ConfigChangedPayload } from "@/lib/types";
import type { SettingsSearchItem } from "@/lib/settingsSearch";
import { SettingsSearch } from "./components/SettingsSearch";
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
  const [servicesSection, setServicesSection] = useState<"translate" | "ocr">("translate");
  const [target, setTarget] = useState<string | null>(null);
  const [dismissedVersion, setDismissedVersion] = useState<string | null>(null);
  const info = useSystemStore((s) => s.info);
  const update = useSystemStore((s) => s.update);
  const setUpdate = useSystemStore((s) => s.setUpdate);
  const loadSystem = useSystemStore((s) => s.load);
  const connections = useSystemStore((s) => s.connections);
  const error = useConfigStore((s) => s.error);
  const setError = useConfigStore((s) => s.setError);
  const load = useConfigStore((s) => s.load);
  const engines = useConfigStore((s) => s.config.translator.engines);
  const defaultId = useConfigStore((s) => s.config.translator.defaultEngineId);

  useEffect(() => {
    void load();
  }, [load]);

  useEffect(() => {
    void loadSystem();
    let active = true;
    const cleanups: (() => void)[] = [];
    const subscribe = <T,>(name: string, handler: (payload: T) => void) => {
      void listen<T>(name, (event) => handler(event.payload)).then((fn) => {
        if (active) cleanups.push(fn); else fn();
      }).catch(setError);
    };
    subscribe("tray://open-about", () => setTab("about"));
    subscribe("tray://check-update", () => {
      setTab("about");
      void emit(Events.ShortcutOpenMain);
      void systemApi.checkUpdate().then(setUpdate).catch(setError);
    });
    subscribe<UpdateInfo>("app://update-checked", setUpdate);
    subscribe<ConfigChangedPayload>(Events.ConfigChanged, (payload) => {
      void loadSystem();
      if (payload.keypaths.some((key) => key === "translator" || key.startsWith("translator.engines") || key === "")) {
        Object.keys(useSystemStore.getState().connections).forEach(useSystemStore.getState().clearConnection);
      }
    });
    subscribe(Events.SelectionPermissionRequired, () => { setTab("general"); void loadSystem(); void emit(Events.ShortcutOpenMain); });
    subscribe("ocr://language-missing", () => { setTab("services"); setServicesSection("ocr"); void loadSystem(); void emit(Events.ShortcutOpenMain); });
    subscribe("ocr://package-required", () => { setTab("services"); setServicesSection("ocr"); void loadSystem(); void emit(Events.ShortcutOpenMain); });
    subscribe<string>("app://operation-error", (message) => {
      setError(message.startsWith("timeout: capture timed out")
        ? t("popup.error.captureTimeout")
        : message.startsWith("timeout: OCR timed out")
          ? t("popup.error.ocrTimeout")
          : message);
    });
    void systemApi.updateState().then((value) => { if (active && value) setUpdate(value); }).catch(setError);
    const onFocus = () => { void loadSystem(); };
    window.addEventListener("focus", onFocus);
    const offError = onLenzError((err) => { setError(err); return true; });
    return () => { active = false; cleanups.forEach((fn) => fn()); offError(); window.removeEventListener("focus", onFocus); };
  }, [setError, loadSystem, setUpdate, t]);

  const selectSetting = (item: SettingsSearchItem) => {
    if (item.section === "translate" || item.section === "ocr") {
      setTab("services"); setServicesSection(item.section);
    } else { setTab(item.section); }
    setTarget(item.id);
  };
  useEffect(() => {
    if (!target) return;
    let frame = 0, attempts = 0;
    const focus = () => {
      const element = document.getElementById(target);
      if (!element && attempts++ < 10) { frame = requestAnimationFrame(focus); return; }
      element?.scrollIntoView({ block: "center" });
      element?.querySelector<HTMLElement>("input,button,select,textarea")?.focus();
      setTarget(null);
    };
    frame = requestAnimationFrame(focus);
    return () => cancelAnimationFrame(frame);
  }, [target, tab, servicesSection]);

  const enabledEngines = engines.filter((e) => e.enabled);
  const def = info ? engines.find((e) => e.id === info.activeEngineId) : enabledEngines.find((e) => e.id === defaultId) ?? enabledEngines[0];
  const connection = def ? connections[def.id] : undefined;
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
          <span className="ml-auto font-mono text-[10px] text-sub">{info?.version ?? "…"}</span>
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
              className={clsx("size-1.5 rounded-full", connection?.ok && "ln-glow")}
              style={{
                background: connection
                  ? connection.ok ? "var(--sage)" : "var(--danger)"
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
          <div className="mt-1">{t(connection ? connection.ok ? "sidebar.connected" : "sidebar.failed" : "sidebar.untested")}</div>
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
          <SettingsSearch onSelect={selectSetting} />
        </header>

        {update?.available && dismissedVersion !== update.latestVersion && <div className="mx-6 mt-3 rounded-md border border-line bg-tint p-3 text-xs">
          <div className="flex items-center justify-between"><span>{t("about.updateAvailable", { version: update.latestVersion })}</span>
            <button type="button" aria-label={t("common.close")} onClick={() => setDismissedVersion(update.latestVersion)}><Icon name="close" size={12} /></button></div>
          <button type="button" className="mt-2 text-accent-ink" onClick={() => { setTab("about"); }}>{t("about.viewUpdate")}</button>
        </div>}
        {error && <div role="alert" className="mx-6 mt-3 rounded-md border border-danger p-3 text-xs text-danger">{error}</div>}

        {tab === "services" ? (
          <div className="min-h-0 flex-1 overflow-hidden px-6 pt-[22px] pb-6">
            <ServicesPane section={servicesSection} onSectionChange={setServicesSection} focusTarget={target} />
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
