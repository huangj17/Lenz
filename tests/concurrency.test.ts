import test from "node:test";
import assert from "node:assert/strict";
import { SerialQueue } from "../src/lib/serialQueue.ts";
import { PopupSessions } from "../src/lib/popupSessions.ts";
import { frameScheduler } from "../src/lib/frameScheduler.ts";
import { errorMessage } from "../src/lib/errors.ts";

function deferred<T>() {
  let resolve!: (value: T) => void;
  const promise = new Promise<T>((r) => { resolve = r; });
  return { promise, resolve };
}

test("queued edit, delete, and reload preserve the latest engine fields", async () => {
  const queue = new SerialQueue();
  const save = deferred<void>();
  let engines = [{ id: "a", name: "old" }, { id: "b", name: "B" }];
  const edit = queue.run(async () => {
    const next = engines.map((e) => e.id === "a" ? { ...e, name: "edited" } : e);
    await save.promise;
    engines = next;
  });
  const remove = queue.run(async () => { engines = engines.filter((e) => e.id !== "b"); });
  const reload = queue.run(async () => [...engines]);
  save.resolve();
  await Promise.all([edit, remove]);
  assert.deepEqual(await reload, [{ id: "a", name: "edited" }]);
});

test("save failure does not block the next operation", async () => {
  const queue = new SerialQueue();
  const failed = queue.run(async () => { throw new Error("disk full"); });
  const next = queue.run(async () => "saved");
  await assert.rejects(failed, /disk full/);
  assert.equal(await next, "saved");
});

test("manual translation ignores delayed older pipeline events", () => {
  const gate = new PopupSessions();
  const events: number[] = [];
  gate.event(1, () => events.push(1));
  const request = gate.begin();
  gate.event(1, () => events.push(-1));
  gate.commit(request, 2);
  gate.event(1, () => events.push(-2));
  gate.event(2, () => events.push(-3));
  gate.event(3, () => events.push(3));
  assert.deepEqual(events, [1, 3]);
});

test("a newer native result arriving during reservation is retained", () => {
  const gate = new PopupSessions();
  const events: string[] = [];
  const old = gate.begin();
  const latest = gate.begin();
  assert.equal(gate.commit(old, 1), false);
  gate.event(3, () => events.push("progress"));
  gate.event(3, () => events.push("result"));
  gate.commit(latest, 2);
  assert.deepEqual(events, ["result"]);
});

test("pointer bursts draw the final coordinates once per frame and cancel cleanly", () => {
  let callback: FrameRequestCallback | undefined;
  let draws = 0;
  let requested = 0;
  let position = 0;
  let rendered = 0;
  const frames = frameScheduler(() => { draws++; rendered = position; }, (fn) => {
    callback = fn; return ++requested;
  }, () => { callback = undefined; });
  for (let i = 0; i < 200; i++) { position = i; frames.schedule(); }
  assert.equal(requested, 1);
  callback!(0);
  assert.equal(draws, 1);
  assert.equal(rendered, 199);
  frames.schedule();
  frames.cancel();
  assert.equal(callback, undefined);
  frames.flush();
  assert.equal(draws, 2);
});

test("structured IPC errors preserve their message", () => {
  assert.equal(errorMessage({ code: "Config", message: "shortcut occupied" }), "shortcut occupied");
  assert.equal(errorMessage(new Error("disk full")), "disk full");
  assert.equal(errorMessage("offline"), "offline");
  assert.equal(errorMessage(null), "Unknown error");
});
