import { useState } from "react";
import { useTranslation } from "react-i18next";
import { clsx } from "clsx";
import { Card, Field } from "../components/primitives";
import { Button } from "@/components/ui/button";
import { Toggle } from "@/components/ui/toggle";
import { Icon } from "@/components/Icon";
import { useConfigStore } from "@/stores/config";

const DEFAULT_PROMPT =
  "你是一位专业译者。请将文本从 {sourceLang} 翻译为 {targetLang}。保留原有格式与术语。";

/** 引擎名首字母作徽标(LM Studio（本地）→ LM) */
function initials(name: string): string {
  const s = name.replace(/[（(].*$/, "").trim();
  return s.slice(0, 2).toUpperCase() || "AI";
}

/** System Prompt 中高亮 {sourceLang} / {targetLang} 变量 */
function HighlightPrompt({ text }: { text: string }) {
  const parts = text.split(/(\{sourceLang\}|\{targetLang\})/g);
  return (
    <>
      {parts.map((p, i) =>
        /^\{(sourceLang|targetLang)\}$/.test(p) ? (
          <span
            key={i}
            className="rounded-[3px] bg-tint px-1 py-px font-mono text-accent-ink"
          >
            {p}
          </span>
        ) : (
          <span key={i}>{p}</span>
        ),
      )}
    </>
  );
}

/** 文本翻译引擎 —— 左列表 + 右编辑器(对齐设计 EnginesPane) */
export function EnginesPane() {
  const { t } = useTranslation();
  const engines = useConfigStore((s) => s.config.translator.engines);
  const defaultId = useConfigStore((s) => s.config.translator.defaultEngineId);
  const [selectedId, setSelectedId] = useState<string | null>(null);
  const selected = engines.find((e) => e.id === selectedId) ?? engines[0];

  return (
    <div className="flex h-full flex-col">
      <div className="mb-[14px] flex items-end justify-between">
        <div className="max-w-[440px] text-xs text-sub">
          {t("engines.desc")}
        </div>
        <Button variant="primary">
          <Icon name="plus" size={13} />
          {t("engines.add")}
        </Button>
      </div>

      {engines.length === 0 ? (
        <div className="flex flex-1 flex-col items-center justify-center gap-3 text-center">
          <div className="flex size-12 items-center justify-center rounded-xl bg-tint text-accent-ink">
            <Icon name="cpu" size={22} />
          </div>
          <div className="text-[13px] font-medium">
            {t("engines.emptyTitle")}
          </div>
          <div className="max-w-[280px] text-xs text-sub">
            {t("engines.emptyDesc")}
          </div>
        </div>
      ) : (
        <div className="grid min-h-0 flex-1 grid-cols-[260px_1fr] gap-[14px]">
          {/* 引擎列表 */}
          <Card>
            {engines.map((e, i) => {
              const active = selected?.id === e.id;
              return (
                <button
                  key={e.id}
                  type="button"
                  onClick={() => setSelectedId(e.id)}
                  className={clsx(
                    "block w-full cursor-pointer border-0 py-2.5 pr-3 pl-[14px] text-left",
                    i < engines.length - 1 && "border-b-[0.5px] border-line",
                  )}
                  style={{
                    background: active ? "var(--tint-soft)" : "transparent",
                    borderLeft: active
                      ? "3px solid var(--lens)"
                      : "3px solid transparent",
                  }}
                >
                  <div className="flex items-center gap-2">
                    <span
                      className={clsx("size-2 rounded-full", e.enabled && "ln-glow")}
                      style={{
                        background: e.enabled
                          ? "var(--sage)"
                          : "color-mix(in srgb, var(--fg) 20%, transparent)",
                      }}
                    />
                    <div className="flex-1 truncate text-[13px] font-medium">
                      {e.name}
                    </div>
                    {e.id === defaultId && (
                      <span className="rounded bg-accent px-1.5 py-px font-mono text-[9.5px] font-semibold uppercase tracking-[0.08em] text-accent-fg">
                        {t("engines.default")}
                      </span>
                    )}
                  </div>
                  {e.model && (
                    <div className="mt-[3px] font-mono text-[11px] text-sub">
                      {e.model}
                    </div>
                  )}
                </button>
              );
            })}
          </Card>

          {/* 引擎编辑器 */}
          {selected && (
            <Card className="overflow-auto p-4">
              <div className="mb-3 flex items-center gap-2.5">
                <div
                  className="flex size-[26px] items-center justify-center rounded-md font-mono text-[11px] font-bold text-white"
                  style={{ background: "linear-gradient(135deg,#4ea3d9,#2d7fb3)" }}
                >
                  {initials(selected.name)}
                </div>
                <div className="flex-1">
                  <div className="text-sm font-semibold">{selected.name}</div>
                  <div className="font-mono text-[11px] text-sub">
                    {selected.kind}
                  </div>
                </div>
                <Toggle checked={selected.enabled} />
              </div>

              <Field label="API URL" value={selected.apiUrl || "—"} mono />
              <Field
                label="API Key"
                value={selected.apiKey ? "sk-•••••••••••••••••••" : "—"}
                mono
              />
              <Field
                label={t("engines.model")}
                value={selected.model || "—"}
                mono
                dropdown
              />

              <div className="mt-2.5">
                <div className="mb-1.5 font-mono text-[11px] uppercase tracking-[0.06em] text-sub">
                  {t("engines.systemPrompt")}
                </div>
                <div className="min-h-[66px] rounded-[7px] border-[0.5px] border-line bg-field p-2.5 text-xs leading-relaxed">
                  <HighlightPrompt text={selected.systemPrompt || DEFAULT_PROMPT} />
                </div>
              </div>

              <div className="mt-4 flex items-center gap-2">
                <Button variant="secondary">{t("engines.test")}</Button>
              </div>
            </Card>
          )}
        </div>
      )}
    </div>
  );
}
