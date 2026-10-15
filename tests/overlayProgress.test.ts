import assert from "node:assert/strict";
import test from "node:test";
import { observeOverlayProgress } from "../src/lib/overlayProgress.ts";
import type { OverlayPayload, OverlayUpdated } from "../src/lib/types.ts";

const snapshot: OverlayPayload = {
  generation: 7, scaleFactor: 2,
  region: { x: 100, y: 100, w: 700, h: 390 },
  viewport: { x: 0, y: 0, w: 1512, h: 982 },
  imageDataUrl: "data:image/png;base64,test",
  progress: { revision: 0, phase: "ocr", blocks: [], error: null },
};
const update = (revision: number, phase: "translate" | "complete", generation = 7): OverlayUpdated => ({
  generation, progress: { revision, phase, blocks: [], error: null },
});
const tick = () => new Promise<void>((resolve) => setImmediate(resolve));

test("overlay shows loading while work is pending and ignores a stale hydration snapshot", async () => {
  let handler!: (value: OverlayUpdated) => void;
  let resolve!: (value: OverlayPayload) => void;
  let current: OverlayPayload | null = null;
  let removed = 0;
  const dispose = observeOverlayProgress(
    async (fn) => { handler = fn; return () => { removed++; }; },
    () => new Promise((done) => { resolve = done; }),
    (value) => { current = value; },
    () => assert.fail("unexpected error"),
  );
  await tick();
  handler(update(1, "translate"));
  resolve(snapshot);
  await tick();
  assert.equal((current as OverlayPayload | null)?.progress.phase, "translate");
  handler(update(0, "translate"));
  handler(update(8, "complete", 6));
  assert.equal((current as OverlayPayload | null)?.progress.phase, "translate");
  handler(update(2, "complete"));
  assert.equal((current as OverlayPayload | null)?.progress.phase, "complete");
  dispose();
  handler(update(3, "translate"));
  assert.equal((current as OverlayPayload | null)?.progress.phase, "complete");
  assert.equal(removed, 1);
});

test("closing before subscription resolves removes the late listener and never loads or publishes", async () => {
  let subscribed!: (remove: () => void) => void;
  let removed = 0;
  const dispose = observeOverlayProgress(
    () => new Promise((done) => { subscribed = done; }),
    async () => { assert.fail("closed overlay must not hydrate"); },
    () => assert.fail("closed overlay must not update"),
    () => assert.fail("closed overlay must not report errors"),
  );
  dispose();
  subscribed(() => { removed++; });
  await tick();
  assert.equal(removed, 1);
});
