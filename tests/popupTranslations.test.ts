import test from "node:test";
import assert from "node:assert/strict";
import { PopupTranslations, type PopupTranslationPanel } from "../src/lib/popupTranslations.ts";
import { PopupSessions } from "../src/lib/popupSessions.ts";
import type { TranslateRequest, TranslateResponse } from "../src/lib/types.ts";

function deferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: unknown) => void;
  const promise = new Promise<T>((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
const input = { sessionId: 1, original: "Hello", sourceLang: null, targetLang: "zh-CN" };
const response = (engineId: string, text = "你好"): TranslateResponse => ({
  translations: [text], detectedSourceLang: "en", engineId, costMs: 25,
});
const flush = async () => { await new Promise<void>((resolve) => setImmediate(resolve)); };
function fixture() {
  const calls: { req: TranslateRequest; sessionId: number; requestId: string; task: ReturnType<typeof deferred<TranslateResponse>> }[] = [];
  const changes: PopupTranslationPanel[] = [];
  const cancelled: { sessionId: number; requestId: string }[] = [];
  const controller = new PopupTranslations((req, sessionId, requestId) => {
    const task = deferred<TranslateResponse>();
    calls.push({ req, sessionId, requestId, task });
    return task.promise;
  }, (panel) => changes.push(panel), async (sessionId, requestId) => { cancelled.push({ sessionId, requestId }); });
  return { controller, calls, changes, cancelled, latest: () => changes.at(-1)! };
}

test("native primary and two extra services settle independently in configuration order", async () => {
  const f = fixture();
  f.controller.start(input, ["google", "apple", "index"], "index", "index");
  assert.deepEqual(f.calls.map((call) => call.req.engineId), ["google", "apple"]);
  assert.ok(f.calls.every((call) => call.sessionId === 1));
  f.calls[1].task.resolve(response("apple"));
  await flush();
  assert.deepEqual(f.latest().rows.map((row) => row.status), ["pending", "success", "pending"]);
  f.controller.success(1, "index", response("index"));
  assert.deepEqual(f.latest().rows.map((row) => row.status), ["pending", "success", "success"]);
  f.calls[0].task.resolve(response("google"));
  await flush();
  assert.deepEqual(f.latest().rows.map((row) => row.engineId), ["google", "apple", "index"]);
  assert.ok(f.latest().rows.every((row) => row.status === "success"));
  assert.equal(f.calls.length, 2, "native primary must not be requested twice");
});

test("a failed native primary cannot hide successful extra services", async () => {
  const f = fixture();
  f.controller.start(input, ["primary", "extra"], "primary", "primary");
  f.calls[0].task.resolve(response("extra"));
  await flush();
  assert.equal(f.controller.nativeFailure(1, { code: "timeout", message: "slow" }), true);
  assert.deepEqual(f.latest().rows.map((row) => row.status), ["error", "success"]);
  f.controller.retry("primary");
  assert.deepEqual(f.calls.map((call) => call.req.engineId), ["extra", "primary"]);
  f.calls[1].task.resolve(response("primary"));
  await flush();
  assert.ok(f.latest().rows.every((row) => row.status === "success"));
});

test("manual retranslation requests each selected service once and retries only the failed row", async () => {
  const f = fixture();
  f.controller.start(input, ["a", "b", "c", "a"], "b");
  assert.deepEqual(f.calls.map((call) => call.req.engineId), ["a", "b", "c"]);
  f.calls[0].task.resolve(response("a"));
  f.calls[1].task.reject({ code: "Network", message: "offline" });
  await flush();
  assert.deepEqual(f.latest().rows.map((row) => row.status), ["success", "error", "pending"]);
  f.controller.retry("b");
  f.controller.retry("b");
  assert.equal(f.calls.length, 4);
  assert.equal(f.calls[3].req.engineId, "b");
  f.calls[2].task.resolve(response("c"));
  f.calls[3].task.resolve(response("b"));
  await flush();
  assert.ok(f.latest().rows.every((row) => row.status === "success"));
});

test("closing the popup suppresses late successes and failures", async () => {
  const f = fixture();
  f.controller.start(input, ["a", "b"], "a");
  f.controller.clear();
  const count = f.changes.length;
  f.calls[0].task.resolve(response("a"));
  f.calls[1].task.reject({ code: "Network", message: "late" });
  await flush();
  assert.equal(f.changes.length, count);
});

test("new text and target language exclude stale responses even if the session number is reused", async () => {
  const f = fixture();
  f.controller.start(input, ["a", "b"], "a");
  f.controller.start({ ...input, original: "Good morning", targetLang: "ja" }, ["a", "b"], "a");
  f.calls[2].task.resolve(response("a", "おはよう"));
  await flush();
  f.calls[0].task.resolve(response("a", "stale"));
  f.calls[1].task.reject(new Error("old request"));
  await flush();
  assert.equal(f.latest().original, "Good morning");
  assert.equal(f.latest().targetLang, "ja");
  assert.deepEqual(f.latest().rows.map((row) => row.status), ["success", "pending"]);
  assert.equal(f.latest().rows[0].status === "success" && f.latest().rows[0].translation, "おはよう");
});

test("cache replay keeps only currently enabled services, including when the cached primary was disabled", async () => {
  const f = fixture();
  f.controller.start(input, ["a", "b"], "old-disabled", "old-disabled");
  f.controller.success(1, "old-disabled", response("old-disabled"));
  f.controller.nativeFailure(1, { code: "failed", message: "old primary" });
  assert.deepEqual(f.calls.map((call) => call.req.engineId), ["a", "b"]);
  assert.deepEqual(f.latest().rows.map((row) => row.status), ["pending", "pending"]);
  f.calls[0].task.resolve(response("a"));
  f.calls[1].task.resolve(response("b"));
  await flush();
  assert.equal(f.latest().rows.length, 2);
  assert.ok(f.latest().rows.every((row) => row.status === "success"));
});

test("close blocks buffered native events and an unfinished manual reservation", () => {
  const sessions = new PopupSessions();
  const changes: number[] = [];
  sessions.event(1, () => changes.push(1));
  const ticket = sessions.begin();
  sessions.event(3, () => changes.push(-3));
  sessions.cancel();
  assert.equal(sessions.commit(ticket, 2), false);
  sessions.event(3, () => changes.push(-30));
  sessions.event(4, () => changes.push(4));
  assert.deepEqual(changes, [1, 4]);
});

test("an extra service's detected language survives a primary response without language detection", async () => {
  const f = fixture();
  f.controller.start(input, ["apple", "primary"], "primary", "primary");
  f.calls[0].task.resolve(response("apple"));
  await flush();
  f.controller.success(1, "primary", { ...response("primary"), detectedSourceLang: null });
  assert.equal(f.latest().sourceLang, "en");
  assert.equal(f.latest().primaryEngineId, "primary");
});


test("remembered folded service skips requests and translates once when expanded", async () => {
  const f = fixture();
  f.controller.start(input, ["a", "b"], "a", undefined, {
    a: { collapsed: true, rememberCollapse: true, skipWhenCollapsed: true },
  });
  assert.deepEqual(f.calls.map((call) => call.req.engineId), ["b"]);
  assert.deepEqual(f.latest().rows.map((row) => row.status), ["idle", "pending"]);
  f.controller.configure("a", { collapsed: false });
  f.controller.configure("a", { collapsed: false });
  assert.deepEqual(f.calls.map((call) => call.req.engineId), ["b", "a"]);
  f.calls[1].task.resolve(response("a"));
  await flush();
  assert.equal(f.latest().rows[0].status, "success");
});

test("temporary folding resets next query and folding without skip preserves translation", async () => {
  const f = fixture();
  const settings = { a: { collapsed: true, rememberCollapse: false, skipWhenCollapsed: true } };
  f.controller.start(input, ["a"], "a", undefined, settings);
  assert.equal(f.latest().rows[0].collapsed, false);
  f.controller.configure("a", { skipWhenCollapsed: false, collapsed: true });
  assert.equal(f.cancelled.length, 0);
  f.calls[0].task.resolve(response("a"));
  await flush();
  assert.equal(f.latest().rows[0].collapsed, true);
  assert.equal(f.latest().rows[0].status, "success");
  f.controller.configure("a", { collapsed: false });
  assert.equal(f.calls.length, 1, "expansion must reuse the completed result");
  f.controller.start({ ...input, sessionId: 2 }, ["a"], "a", undefined, settings);
  assert.equal(f.latest().rows[0].collapsed, false);
  assert.equal(f.calls.length, 2);
});

test("folding cancels only that service and excludes its late result after expansion", async () => {
  const f = fixture();
  f.controller.start(input, ["a", "b"], "a");
  f.controller.configure("a", { skipWhenCollapsed: true, collapsed: true });
  assert.deepEqual(f.cancelled, [{ sessionId: 1, requestId: f.calls[0].requestId }]);
  assert.deepEqual(f.latest().rows.map((row) => row.status), ["idle", "pending"]);
  f.controller.configure("a", { collapsed: false });
  assert.notEqual(f.calls[2].requestId, f.calls[0].requestId);
  f.calls[2].task.resolve(response("a", "fresh"));
  f.calls[1].task.resolve(response("b"));
  await flush();
  f.calls[0].task.resolve(response("a", "stale"));
  await flush();
  const row = f.latest().rows[0];
  assert.equal(row.status === "success" && row.translation, "fresh");
  assert.equal(f.latest().rows[1].status, "success");
});

test("native primary can be cancelled independently and resumed through a new request", async () => {
  const f = fixture();
  f.controller.start(input, ["a", "b"], "a", "a", {}, "native-a");
  f.controller.configure("a", { collapsed: true, skipWhenCollapsed: true });
  assert.deepEqual(f.cancelled, [{ sessionId: 1, requestId: "native-a" }]);
  f.controller.nativeSuccess(1, "a", response("a", "stale native"));
  assert.equal(f.latest().rows[0].status, "idle");
  f.controller.configure("a", { collapsed: false });
  f.calls[1].task.resolve(response("a", "fresh"));
  await flush();
  f.controller.nativeFailure(1, { code: "timeout", message: "late native" });
  const row = f.latest().rows[0];
  assert.equal(row.status === "success" && row.translation, "fresh");
  assert.equal(f.latest().rows[1].status, "pending");
});

test("changing skip policy while folded cancels or starts only that row", () => {
  const f = fixture();
  f.controller.start(input, ["a", "b"], "a");
  f.controller.configure("a", { collapsed: true });
  assert.equal(f.cancelled.length, 0);
  f.controller.configure("a", { skipWhenCollapsed: true });
  assert.equal(f.cancelled.length, 1);
  assert.equal(f.latest().rows[0].status, "idle");
  f.controller.configure("a", { skipWhenCollapsed: false });
  assert.equal(f.calls.length, 3);
  assert.equal(f.calls[2].req.engineId, "a");
  assert.equal(f.latest().rows[0].collapsed, true);
  assert.equal(f.latest().rows[0].status, "pending");
});

test("all folded rows stay idle, and optimistic native requests are cancelled", () => {
  const f = fixture();
  f.controller.start(input, ["a", "b"], "a", "a", {
    a: { collapsed: true, rememberCollapse: true, skipWhenCollapsed: true },
    b: { collapsed: true, rememberCollapse: true, skipWhenCollapsed: true },
  }, "native-before-save");
  assert.equal(f.calls.length, 0);
  assert.ok(f.latest().rows.every((row) => row.status === "idle"));
  assert.deepEqual(f.cancelled, [{ sessionId: 1, requestId: "native-before-save" }]);
});
