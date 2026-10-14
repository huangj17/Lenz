import { useState } from "react";
import { useTranslation } from "react-i18next";
import { Icon } from "@/components/Icon";
import { searchSettings, type SettingsSearchItem, type SettingsSection } from "@/lib/settingsSearch";

const INDEX: [string, SettingsSection, string, string][] = [
  ["general-auto-start", "general", "general.autoStart", "启动 登录 startup login autostart"],
  ["general-update", "general", "general.checkUpdate", "更新 版本 update version"],
  ["general-open-on-launch", "general", "general.openOnLaunch", "启动 窗口 launch window"],
  ["selection-indicator", "general", "general.selectionIndicator", "划词 图标 权限 selection indicator accessibility permission"],
  ["theme", "general", "theme.label", "外观 深色 浅色 dark light appearance theme"],
  ["locale", "general", "locale.label", "语言 中文 英文 locale language chinese english"],
  ["shortcuts", "shortcuts", "nav.shortcuts", "快捷键 录制 清空 settings comma keyboard shortcut"],
  ["engine-timeout", "translate", "engines.timeout", "超时 时间 timeout milliseconds"],
  ["engines", "translate", "services.translate", "翻译 引擎 服务 模型 密钥 prompt temperature tokens openai ollama api key model"],
  ["ocr-accuracy", "ocr", "ocr.accuracy", "识别 精度 快速 精准 confidence accuracy fast"],
  ["ocr-languages", "ocr", "ocr.languages", "OCR 语言 自动 检测 language detection"],
  ["about", "about", "nav.about", "关于 许可证 更新日志 版本 github license release changelog about version"],
];
export function SettingsSearch({ onSelect }: { onSelect: (item: SettingsSearchItem) => void }) {
  const { t } = useTranslation();
  const [query, setQuery] = useState("");
  const [open, setOpen] = useState(false);
  const [activeIndex, setActiveIndex] = useState(0);
  const results = searchSettings(INDEX.map(([id, section, key, keywords]) => ({ id, section, label: t(key), keywords })), query);
  const select = (item: SettingsSearchItem) => { onSelect(item); setQuery(""); setOpen(false); };
  return (
    <div className="relative">
      <div className="flex items-center gap-1.5 rounded-md border-[0.5px] border-line bg-field py-1 px-2 text-xs">
        <Icon name="search" size={13} />
        <input value={query} role="combobox" aria-expanded={open && !!query.trim()} aria-controls="settings-search-results"
          aria-activedescendant={results[activeIndex] ? "search-result-" + results[activeIndex].id : undefined}
          placeholder={t("header.search")} aria-label={t("header.search")}
          className="w-36 bg-transparent outline-none"
          onChange={(event) => { setQuery(event.target.value); setOpen(true); setActiveIndex(0); }}
          onFocus={() => setOpen(true)}
          onBlur={() => setOpen(false)}
          onKeyDown={(event) => {
            if (event.key === "Escape") { event.stopPropagation(); setOpen(false); }
            if (event.key === "ArrowDown" && results.length) { event.preventDefault(); setActiveIndex((i) => (i + 1) % results.length); }
            if (event.key === "ArrowUp" && results.length) { event.preventDefault(); setActiveIndex((i) => (i - 1 + results.length) % results.length); }
            if (event.key === "Enter" && results[activeIndex]) { event.preventDefault(); select(results[activeIndex]); }
          }} />
      </div>
      {open && query.trim() && <div id="settings-search-results" role="listbox" aria-label={t("header.search")} className="absolute right-0 top-full z-50 mt-1 w-64 rounded-md border border-line bg-card p-1 shadow-lg">
        {results.length ? results.map((item, index) => <button key={item.id} id={"search-result-" + item.id} type="button" role="option" aria-selected={index === activeIndex}
          className={"block w-full rounded px-3 py-2 text-left text-xs hover:bg-tint " + (index === activeIndex ? "bg-tint" : "")}
          onMouseDown={(event) => event.preventDefault()} onClick={() => select(item)}>
          {item.label}
        </button>) : <div className="p-3 text-xs text-sub">{t("header.noResults")}</div>}
      </div>}
    </div>
  );
}
