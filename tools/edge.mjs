// Microsoft Edge, headless, driven through its DevTools protocol: shared by the scripts that look
// at the web builds (tutorial screenshots, layout check). No dependency: Node's own WebSocket.

import { spawn } from 'node:child_process';
import { existsSync, mkdtempSync, readFileSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

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
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
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
  return { send, close };
}
