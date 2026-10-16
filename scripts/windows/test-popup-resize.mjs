// Windows CI only. Debugging is enabled in this child process, never in the installer.
// https://learn.microsoft.com/microsoft-edge/webview2/how-to/debug-visual-studio-code
import { spawn, execFileSync } from "node:child_process";
import { writeFile } from "node:fs/promises";
import { createServer } from "node:net";
import { parseArgs } from "node:util";
import { setTimeout as delay } from "node:timers/promises";

const { values } = parseArgs({ options: {
  binary: { type: "string" }, report: { type: "string" },
} });
if (process.platform !== "win32" || !values.binary || !values.report) {
  throw new Error("Run on Windows with --binary <installed lenz.exe> --report <json>");
}
const listener = createServer();
await new Promise((resolve) => listener.listen(0, "127.0.0.1", resolve));
const port = listener.address().port;
await new Promise((resolve) => listener.close(resolve));
const child = spawn(values.binary, ["--show-main"], { env: {
  ...process.env,
  LENZ_STARTUP_NO_DIALOG: "1",
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-address=127.0.0.1 --remote-debugging-port=${port}`,
} });
const report = { passed: false };
let socket;
let output = "";
let spawnError;
child.on("error", (error) => { spawnError = error; });
child.stdout.on("data", (data) => { output = (output + data).slice(-16000); });
child.stderr.on("data", (data) => { output = (output + data).slice(-16000); });
try {
  let page;
  const deadline = Date.now() + 60000;
  while (!page && Date.now() < deadline) {
    if (spawnError) throw spawnError;
    if (child.exitCode !== null) throw new Error(`Application exited: ${child.exitCode}`);
    try {
      const response = await fetch(`http://127.0.0.1:${port}/json/list`, { signal: AbortSignal.timeout(2000) });
      page = (await response.json()).find((target) => target.url.includes("window=popup"));
    } catch { /* WebView2 is still starting. */ }
    if (!page) await delay(250);
  }
  if (!page) throw new Error("Popup WebView2 target did not become available");
  socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((resolve, reject) => {
    socket.addEventListener("open", resolve, { once: true });
    socket.addEventListener("error", reject, { once: true });
  });
  let nextId = 0;
  const pending = new Map();
  socket.addEventListener("message", ({ data }) => {
    const message = JSON.parse(data);
    const handler = pending.get(message.id);
    if (!handler) return;
    pending.delete(message.id);
    clearTimeout(handler.timer);
    if (message.error) handler.reject(new Error(JSON.stringify(message.error)));
    else handler.resolve(message.result);
  });
  function evaluate(expression) {
    return new Promise((resolve, reject) => {
      const id = ++nextId;
      const timer = setTimeout(() => { pending.delete(id); reject(new Error("Popup evaluation timed out")); }, 90000);
      pending.set(id, { resolve, reject, timer });
      socket.send(JSON.stringify({ id, method: "Runtime.evaluate", params: {
        expression, awaitPromise: true, returnByValue: true,
      } }));
    }).then((result) => {
      if (result.exceptionDetails) throw new Error(JSON.stringify(result.exceptionDetails));
      return result.result.value;
    });
  }
  while (!(await evaluate("Boolean(window.__TAURI_INTERNALS__ && document.querySelector('.ln-popup'))"))) {
    if (Date.now() > deadline) throw new Error("Popup frontend did not become ready");
    await delay(250);
  }
  report.dimensions = await evaluate(`(async () => {
    const invoke = window.__TAURI_INTERNALS__.invoke;
    const label = 'popup';
    const scale = await invoke('plugin:window|scale_factor', { label });
    const tolerance = 1 / scale + 0.01;
    const size = async () => {
      const physical = await invoke('plugin:window|inner_size', { label });
      return { width: physical.width / scale, height: physical.height / scale };
    };
    const setSize = (width, height) => invoke('plugin:window|set_size', {
      label, value: { Logical: { width, height } },
    });
    const frames = () => new Promise(resolve => requestAnimationFrame(() => requestAnimationFrame(resolve)));
    const card = document.querySelector('.ln-popup');
    // Keep the real ResizeObserver active. Synthetic card heights represent content updates;
    // no OCR, translation service, or configuration writes are involved.
    card.style.setProperty('min-height', '0', 'important');
    card.style.setProperty('height', '220px', 'important');
    await invoke('plugin:window|show', { label });
    await setSize(360, 220);
    await frames();
    const baseline = await size();
    let heightUpdates = 0;
    for (const width of [300, 360, 537, 720]) {
      await setSize(width, 220);
      for (let iteration = 0; iteration < 100; iteration++) {
        const height = [96, 150, 480, 220][iteration % 4];
        card.style.setProperty('height', height + 'px', 'important');
        await invoke('resize_popup', { height });
        await frames();
        const actual = await size();
        if (Math.abs(actual.width - width) > tolerance || Math.abs(actual.height - height) > tolerance) {
          throw Error('Height update ' + iteration + ': expected ' + width + 'x' + height + ', got ' + JSON.stringify(actual));
        }
        heightUpdates++;
      }
    }
    card.style.setProperty('height', '220px', 'important');
    await invoke('resize_popup', { height: 220 });
    await frames();
    for (let iteration = 0; iteration < 100; iteration++) {
      const width = [200, 537, 900, 360][iteration % 4];
      await invoke('resize_popup_width', { width });
      await frames();
      const actual = await size();
      if (Math.abs(actual.width - Math.max(300, Math.min(720, width))) > tolerance || Math.abs(actual.height - 220) > tolerance) {
        throw Error('Width update ' + iteration + ': got ' + JSON.stringify(actual));
      }
    }
    return { scale, baseline, heightUpdates, widthUpdates: 100, final: await size() };
  })()`);
  report.passed = true;
} catch (error) {
  report.error = String(error.stack || error);
  report.processOutput = output;
  process.exitCode = 1;
} finally {
  socket?.close();
  if (child.pid && child.exitCode === null) {
    try { execFileSync("taskkill", ["/PID", String(child.pid), "/T", "/F"], { stdio: "ignore" }); }
    catch { child.kill(); }
  }
  await writeFile(values.report, JSON.stringify(report, null, 2) + "\n");
  console.log(JSON.stringify(report, null, 2));
}
