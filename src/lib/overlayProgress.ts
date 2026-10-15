import type { OverlayPayload, OverlayUpdated } from "./types";

function applyProgress(payload: OverlayPayload, update: OverlayUpdated | null): OverlayPayload {
  if (!update || update.generation !== payload.generation
    || update.progress.revision <= payload.progress.revision) return payload;
  return { ...payload, progress: update.progress };
}

/** 先订阅再取快照;快照返回期间到达的进度不能被旧快照覆盖。 */
export function observeOverlayProgress(
  subscribe: (handler: (update: OverlayUpdated) => void) => Promise<() => void>,
  load: () => Promise<OverlayPayload | null>,
  onPayload: (payload: OverlayPayload | null) => void,
  onError: () => void,
): () => void {
  let active = true;
  let unlisten: (() => void) | undefined;
  let payload: OverlayPayload | null = null;
  let pending: OverlayUpdated | null = null;
  void (async () => {
    unlisten = await subscribe((update) => {
      if (!active) return;
      if (!payload) {
        if (!pending || update.generation > pending.generation
          || (update.generation === pending.generation && update.progress.revision > pending.progress.revision)) {
          pending = update;
        }
        return;
      }
      const next = applyProgress(payload, update);
      if (next === payload) return;
      payload = next;
      onPayload(payload);
    });
    if (!active) { unlisten(); return; }
    const snapshot = await load();
    if (!active) return;
    payload = snapshot ? applyProgress(snapshot, pending) : null;
    onPayload(payload);
  })().catch(() => { if (active) onError(); });
  return () => { active = false; unlisten?.(); };
}
