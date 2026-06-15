import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import {
  Settings,
  Keyboard,
  Languages,
  ScanText,
  Info,
  type LucideIcon,
} from "lucide-react";
import { Tabs, TabsList, TabsTrigger, TabsContent } from "@/components/ui/tabs";
import { useConfigStore } from "@/stores/config";
import { GeneralTab } from "./tabs/GeneralTab";
import { ShortcutsTab } from "./tabs/ShortcutsTab";
import { EnginesTab } from "./tabs/EnginesTab";
import { OcrTab } from "./tabs/OcrTab";
import { AboutTab } from "./tabs/AboutTab";

type TabId = "general" | "shortcuts" | "engines" | "ocr" | "about";

const NAV: { id: TabId; icon: LucideIcon; labelKey: string }[] = [
  { id: "general", icon: Settings, labelKey: "nav.general" },
  { id: "shortcuts", icon: Keyboard, labelKey: "nav.shortcuts" },
  { id: "engines", icon: Languages, labelKey: "nav.engines" },
  { id: "ocr", icon: ScanText, labelKey: "nav.ocr" },
  { id: "about", icon: Info, labelKey: "nav.about" },
];

/**
 * 设置主窗口:左侧 Tab 导航(通用 / 快捷键 / 翻译引擎 / OCR / 关于)+ 右侧内容。
 * 无历史 Tab(历史记录功能已删除)。
 *
 * macOS 顶部留出可拖拽区(data-tauri-drag-region),适配 titleBarStyle overlay。
 */
export function MainWindow() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<TabId>("general");
  const load = useConfigStore((s) => s.load);

  useEffect(() => {
    void load();
  }, [load]);

  return (
    <Tabs
      value={tab}
      onValueChange={(v) => setTab(v as TabId)}
      orientation="vertical"
      className="grid h-screen grid-cols-[200px_1fr] bg-bg text-fg"
    >
      {/* 侧边栏 */}
      <aside className="flex flex-col gap-1 border-r border-border bg-bg-secondary/40 p-2.5">
        <div
          data-tauri-drag-region
          className="flex items-center gap-2 px-2 pt-2 pb-3"
        >
          <div className="size-5 rounded-md bg-accent" />
          <span className="text-sm font-semibold tracking-tight">
            {t("app.name")}
          </span>
          <span className="ml-auto font-mono text-[10px] text-fg-muted">
            0.1.0
          </span>
        </div>
        <TabsList aria-label="Settings sections">
          {NAV.map(({ id, icon: Icon, labelKey }) => (
            <TabsTrigger key={id} value={id}>
              <Icon />
              <span>{t(labelKey)}</span>
            </TabsTrigger>
          ))}
        </TabsList>
      </aside>

      {/* 内容区 */}
      <main className="flex min-h-0 flex-col">
        <header
          data-tauri-drag-region
          className="flex h-12 items-center border-b border-border px-6 text-[15px] font-semibold"
        >
          {t(NAV.find((n) => n.id === tab)?.labelKey ?? "nav.general")}
        </header>
        <div className="min-h-0 flex-1 overflow-y-auto px-6 py-5">
          <TabsContent value="general">
            <GeneralTab />
          </TabsContent>
          <TabsContent value="shortcuts">
            <ShortcutsTab />
          </TabsContent>
          <TabsContent value="engines">
            <EnginesTab />
          </TabsContent>
          <TabsContent value="ocr">
            <OcrTab />
          </TabsContent>
          <TabsContent value="about">
            <AboutTab />
          </TabsContent>
        </div>
      </main>
    </Tabs>
  );
}

export default MainWindow;
