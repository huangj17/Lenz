import { errorMessage } from "./errors.ts";
import type { PopupServiceUiConfig, ScreenshotTranslatingPayload, TranslateRequest, TranslateResponse } from "./types";

export type PopupTranslationInput = Omit<ScreenshotTranslatingPayload, "engineId" | "sessionId" | "requestId"> & { sessionId: number };
export type PopupTranslationError = { code: string; message: string };
export type PopupTranslationRow = { engineId: string } & Required<PopupServiceUiConfig> & (
  | { status: "idle" }
  | { status: "pending" }
  | { status: "success"; translation: string; costMs: number; sourceLang: string | null }
  | { status: "error"; error: PopupTranslationError }
);
export type PopupTranslationPanel = PopupTranslationInput & {
  kind: "translations";
  primaryEngineId: string;
  rows: PopupTranslationRow[];
};

export function popupTranslationRows(engineIds: string[], settings: Record<string, PopupServiceUiConfig> = {}): PopupTranslationRow[] {
  return [...new Set(engineIds.filter(Boolean))].map((engineId) => {
    const policy = settings[engineId];
    const rememberCollapse = policy?.rememberCollapse ?? false;
    const collapsed = rememberCollapse && (policy?.collapsed ?? false);
    const skipWhenCollapsed = policy?.skipWhenCollapsed ?? false;
    return { engineId, collapsed, rememberCollapse, skipWhenCollapsed,
      status: collapsed && skipWhenCollapsed ? "idle" : "pending" };
  });
}

/** One popup session, with independently settling rows in configuration order. */
export class PopupTranslations {
  private epoch = 0;
  private panel: PopupTranslationPanel | undefined;
  private input: PopupTranslationInput | undefined;
  private nativeEngineId: string | undefined;
  private nativeCancelled = false;
  private active = new Map<string, string>();
  private readonly request: (req: TranslateRequest, sessionId: number, requestId: string) => Promise<TranslateResponse>;
  private readonly changed: (panel: PopupTranslationPanel) => void;
  private readonly cancel: (sessionId: number, requestId: string) => Promise<void>;

  constructor(
    request: (req: TranslateRequest, sessionId: number, requestId: string) => Promise<TranslateResponse>,
    changed: (panel: PopupTranslationPanel) => void,
    cancel: (sessionId: number, requestId: string) => Promise<void> = async () => {},
  ) {
    this.request = request;
    this.changed = changed;
    this.cancel = cancel;
  }

  clear(): void {
    this.epoch++;
    for (const id of [...this.active.keys()]) this.cancelRow(id);
    this.panel = undefined;
    this.input = undefined;
    this.nativeEngineId = undefined;
    this.nativeCancelled = false;
  }

  matches(input: PopupTranslationInput): boolean {
    const panel = this.panel;
    return !!panel && panel.sessionId === input.sessionId
      && panel.original === input.original && panel.targetLang === input.targetLang;
  }

  start(input: PopupTranslationInput, engineIds: string[], preferredId: string, nativeEngineId?: string,
    settings: Record<string, PopupServiceUiConfig> = {}, nativeRequestId?: string | null): void {
    this.clear();
    const ids = [...new Set(engineIds.filter(Boolean))];
    this.input = input;
    this.nativeEngineId = nativeEngineId;
    this.panel = {
      ...input,
      kind: "translations",
      primaryEngineId: ids.includes(preferredId) ? preferredId : ids[0] ?? "",
      rows: popupTranslationRows(ids, settings),
    };
    this.changed(this.panel);
    if (nativeEngineId && nativeRequestId) {
      this.active.set(nativeEngineId, nativeRequestId);
      if (!this.panel.rows.some((row) => row.engineId === nativeEngineId && row.status === "pending")) {
        this.cancelRow(nativeEngineId);
      }
    }
    // The native F1/F2 pipeline already owns the primary request. Joining its
    // session lets extra requests share cancellation without translating it twice.
    for (const row of this.panel.rows) if (row.engineId !== nativeEngineId && row.status === "pending") this.launch(row.engineId);
  }

  configure(engineId: string, changes: Partial<PopupServiceUiConfig>): void {
    const current = this.panel?.rows.find((row) => row.engineId === engineId);
    if (!current) return;
    let row: PopupTranslationRow = { ...current, ...changes };
    if (row.collapsed && row.skipWhenCollapsed && row.status === "pending") {
      this.cancelRow(engineId);
      row = { ...row, status: "idle" };
    }
    this.settle(row);
    if (row.status === "idle" && (!row.collapsed || !row.skipWhenCollapsed)) this.launch(engineId);
  }

  nativeSuccess(sessionId: number, engineId: string, result: TranslateResponse): void {
    if (this.nativeCancelled || this.panel?.sessionId !== sessionId) return;
    this.active.delete(engineId);
    this.success(sessionId, engineId, result);
  }

  success(sessionId: number, engineId: string, result: TranslateResponse): void {
    if (this.panel?.sessionId !== sessionId) return;
    this.settle({
      engineId, status: "success", translation: result.translations[0] ?? "",
      costMs: result.costMs, sourceLang: result.detectedSourceLang,
    });
  }

  failure(sessionId: number, engineId: string, error: PopupTranslationError): boolean {
    if (this.panel?.sessionId !== sessionId) return false;
    this.settle({ engineId, status: "error", error });
    return true;
  }

  nativeFailure(sessionId: number, error: PopupTranslationError): boolean {
    if (this.panel?.sessionId !== sessionId) return false;
    if (this.nativeEngineId && !this.nativeCancelled) {
      this.active.delete(this.nativeEngineId);
      this.failure(sessionId, this.nativeEngineId, error);
    }
    return true;
  }

  retry(engineId: string): void {
    if (this.panel?.rows.some((row) => row.engineId === engineId && row.status === "error")) {
      this.settle({ engineId, status: "pending" });
      this.launch(engineId);
    }
  }

  private launch(engineId: string): void {
    const input = this.input;
    if (!input || this.active.has(engineId)) return;
    const epoch = this.epoch;
    const requestId = crypto.randomUUID();
    this.active.set(engineId, requestId);
    this.settle({ engineId, status: "pending" });
    void this.request({
      texts: [input.original], sourceLang: input.sourceLang ?? undefined, targetLang: input.targetLang, engineId,
    }, input.sessionId, requestId).then((result) => {
      if (epoch !== this.epoch || this.active.get(engineId) !== requestId) return;
      this.active.delete(engineId);
      this.success(input.sessionId, engineId, result);
    }).catch((error: unknown) => {
      if (epoch !== this.epoch || this.active.get(engineId) !== requestId) return;
      this.active.delete(engineId);
      const code = typeof error === "object" && error !== null && "code" in error
        ? String(error.code) : "TranslateFailed";
      if (code !== "Cancelled") this.failure(input.sessionId, engineId, { code, message: errorMessage(error) });
    });
  }

  private cancelRow(engineId: string): void {
    const requestId = this.active.get(engineId);
    this.active.delete(engineId);
    if (engineId === this.nativeEngineId) this.nativeCancelled = true;
    if (requestId && this.input) void this.cancel(this.input.sessionId, requestId).catch(() => {});
  }

  private settle(update: { engineId: string } & (
    | { status: "idle" | "pending" }
    | { status: "success"; translation: string; costMs: number; sourceLang: string | null }
    | { status: "error"; error: PopupTranslationError }
  )): void {
    const panel = this.panel;
    const current = panel?.rows.find((row) => row.engineId === update.engineId);
    if (!panel || !current) return;
    const row = { ...current, ...update } as PopupTranslationRow;
    this.panel = {
      ...panel,
      sourceLang: row.status === "success" && row.sourceLang
        && (row.engineId === panel.primaryEngineId || !panel.sourceLang) ? row.sourceLang : panel.sourceLang,
      rows: panel.rows.map((current) => current.engineId === row.engineId ? row : current),
    };
    this.changed(this.panel);
  }
}
