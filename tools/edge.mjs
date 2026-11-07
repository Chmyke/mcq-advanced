// Microsoft Edge, headless, driven through its DevTools protocol: shared by the scripts that look
// at the web builds (tutorial screenshots, layout check, web check). No dependency: Node's own
// WebSocket and http server.

import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { createServer } from 'node:http';
import { tmpdir } from 'node:os';
import { extname, join, normalize, sep } from 'node:path';

/// The Edge executable, or undefined when it is not installed.
export const EDGE = [
  join(process.env['ProgramFiles(x86)'] ?? '', 'Microsoft/Edge/Application/msedge.exe'),
  join(process.env.ProgramFiles ?? '', 'Microsoft/Edge/Application/msedge.exe'),
].find((p) => existsSync(p));

export const sleep = (ms) => new Promise((ok) => setTimeout(ok, ms));

// A fresh Edge on a blank page, in a throwaway profile; `send` calls a DevTools method.
export async function launch(width, height) {
  if (!EDGE) throw new Error('Microsoft Edge is required.');
  const profile = mkdtempSync(join(tmpdir(), 'mcq-edge-'));
  const edge = spawn(EDGE, [
    '--headless=new', '--remote-debugging-port=0', `--user-data-dir=${profile}`, '--no-first-run',
    '--hide-scrollbars', '--force-color-profile=srgb', `--window-size=${width},${height}`, 'about:blank',
  ], { stdio: 'ignore' });
  const portFile = join(profile, 'DevToolsActivePort');
  for (let i = 0; i < 100 && !existsSync(portFile); i++) await sleep(100);
  const port = readFileSync(portFile, 'utf8').split('\n')[0];
  const targets = await (await fetch(`http://127.0.0.1:${port}/json/list`)).json();
  const page = targets.find((t) => t.type === 'page');
  const socket = new WebSocket(page.webSocketDebuggerUrl);
  await new Promise((ok, ko) => { socket.onopen = ok; socket.onerror = ko; });
  let next = 0;
  const waiting = new Map();
  // A script never waits on a page's own dialog (leave the page?, confirm): each is accepted
  // and counted.
  let dialogs = 0;
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (message.method === 'Page.javascriptDialogOpening') {
      dialogs++;
      socket.send(JSON.stringify({ id: ++next, method: 'Page.handleJavaScriptDialog', params: { accept: true } }));
      return;
    }
    const pending = waiting.get(message.id);
    if (!pending) return;
    waiting.delete(message.id);
    if (message.error) pending.ko(new Error(message.error.message));
    else pending.ok(message.result);
  };
  const send = (method, params = {}) => new Promise((ok, ko) => {
    const id = ++next;
    waiting.set(id, { ok, ko });
    socket.send(JSON.stringify({ id, method, params }));
  });
  const close = async () => {
    socket.close();
    edge.kill();
    await sleep(300);
    rmSync(profile, { recursive: true, force: true, maxRetries: 5, retryDelay: 200 });
  };
  return { send, close, dialogs: () => dialogs };
}

const TYPES = {
  '.html': 'text/html; charset=utf-8', '.js': 'text/javascript', '.wasm': 'application/wasm',
  '.css': 'text/css', '.svg': 'image/svg+xml', '.json': 'application/json', '.txt': 'text/plain',
};

// A local web server for a folder, on a free port of 127.0.0.1; resolves to the server.
export function serve(root) {
  const server = createServer((request, response) => {
    let path = decodeURIComponent(new URL(request.url, 'http://x').pathname);
    if (path.endsWith('/')) path += 'index.html';
    const file = normalize(join(root, path));
    if (!file.startsWith(root + sep) || !existsSync(file)) {
      response.writeHead(404).end();
      return;
    }
    response.writeHead(200, { 'Content-Type': TYPES[extname(file)] ?? 'application/octet-stream' });
    response.end(readFileSync(file));
  });
  return new Promise((ok) => server.listen(0, '127.0.0.1', () => ok(server)));
}

// A page driven step by step: `run` evaluates JavaScript in it (promises awaited) and returns
// the value; `waitFor` waits up to 15 s for an expression to hold; `click` and `type` act on an
// element once it is there.
export function steps(send) {
  const run = async (expression) => {
    const { result, exceptionDetails } = await send('Runtime.evaluate', { expression, awaitPromise: true, returnByValue: true });
    if (exceptionDetails) throw new Error(`${exceptionDetails.exception?.description ?? exceptionDetails.text}: ${expression}`);
    return result.value;
  };
  const waitFor = async (expression, what) => {
    for (let i = 0; i < 150; i++) {
      // A page still loading has no context to evaluate in yet: try again.
      if (await run(`!!(${expression})`).catch(() => false)) return;
      await sleep(100);
    }
    throw new Error(`Not on screen: ${what}`);
  };
  const click = async (expression, what) => {
    await waitFor(expression, what);
    await run(`(${expression}).click()`);
    await sleep(250);
  };
  const type = async (expression, text, what) => {
    await waitFor(expression, what);
    await run(`(${expression}).focus()`);
    await send('Input.insertText', { text });
    await sleep(120);
  };
  return { run, waitFor, click, type };
}
