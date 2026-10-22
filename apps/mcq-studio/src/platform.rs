//! What the Studio needs from its host beyond the player's helpers: dropped and pasted files, the
//! microphone, the draft kept in the browser, confirmation and printing.
//!
//! Data goes through the eval channel (`dioxus.send` / `dioxus.recv`), never spliced into
//! scripts, so nothing typed or imported can be executed.

use dioxus::prelude::*;

pub use mcq_player::platform::{browser_language, download, prefers_dark, rng};

/// Sends `[file name, base 64]` for every file dropped on the page or pasted into it.
pub const DROP_AND_PASTE_SCRIPT: &str = r#"
const send = (f) => {
  const r = new FileReader();
  r.onload = () => { const s = r.result; dioxus.send([f.name || 'pasted', s.slice(s.indexOf(',') + 1)]); };
  r.readAsDataURL(f);
};
const hasFiles = (e) => e.dataTransfer && Array.from(e.dataTransfer.types || []).includes('Files');
document.addEventListener('dragover', (e) => { if (hasFiles(e)) { e.preventDefault(); document.body.classList.add('dropping'); } });
document.addEventListener('dragleave', (e) => { if (!e.relatedTarget) document.body.classList.remove('dropping'); });
document.addEventListener('drop', (e) => {
  document.body.classList.remove('dropping');
  if (!hasFiles(e)) return;
  e.preventDefault();
  for (const f of e.dataTransfer.files) send(f);
});
document.addEventListener('paste', (e) => {
  const files = e.clipboardData ? Array.from(e.clipboardData.files) : [];
  if (!files.length) return;
  e.preventDefault();
  for (const f of files) send(f);
});
await new Promise(() => {});
"#;

/// Records the microphone until `STOP_RECORDING_SCRIPT` runs, then returns
/// `[file extension, base 64]`, or `['', '']` when the microphone is refused or missing.
pub const RECORD_SCRIPT: &str = r#"
let stream;
try { stream = await navigator.mediaDevices.getUserMedia({ audio: true }); } catch (e) { return ['', '']; }
const kinds = [['audio/ogg;codecs=opus', 'ogg'], ['audio/webm;codecs=opus', 'weba'], ['audio/webm', 'weba'], ['audio/mp4', 'm4a']];
const kind = kinds.find(([t]) => window.MediaRecorder && MediaRecorder.isTypeSupported(t));
if (!kind) { stream.getTracks().forEach((t) => t.stop()); return ['', '']; }
const rec = new MediaRecorder(stream, { mimeType: kind[0] });
const chunks = [];
rec.ondataavailable = (e) => { if (e.data.size) chunks.push(e.data); };
const stopped = new Promise((r) => { rec.onstop = r; });
rec.start();
await new Promise((r) => { window.mcqStopRecording = r; });
window.mcqStopRecording = null;
rec.stop();
await stopped;
stream.getTracks().forEach((t) => t.stop());
const blob = new Blob(chunks, { type: kind[0] });
const url = await new Promise((r) => { const f = new FileReader(); f.onload = () => r(f.result); f.readAsDataURL(blob); });
return [kind[1], url.slice(url.indexOf(',') + 1)];
"#;

pub const STOP_RECORDING_SCRIPT: &str = "if (window.mcqStopRecording) window.mcqStopRecording();";

const OPEN_DB: &str = r#"
const db = await new Promise((ok, ko) => {
  const r = indexedDB.open('mcq-studio', 1);
  r.onupgradeneeded = () => r.result.createObjectStore('drafts');
  r.onsuccess = () => ok(r.result);
  r.onerror = () => ko(r.error);
});
"#;

/// Keeps the draft (a `.qcmz` in base 64) in the browser's database: no size limit to speak of,
/// unlike `localStorage`.
pub async fn save_draft(base64: String) -> bool {
    let script = format!(
        "{OPEN_DB}
const data = await dioxus.recv();
await new Promise((ok, ko) => {{
  const t = db.transaction('drafts', 'readwrite');
  t.objectStore('drafts').put(data, 'current');
  t.oncomplete = ok; t.onerror = () => ko(t.error);
}});
return true;"
    );
    let wrapped = format!("try {{ {script} }} catch (e) {{ return false; }}");
    let eval = document::eval(&wrapped);
    if eval.send(base64).is_err() {
        return false;
    }
    eval.join::<bool>().await.unwrap_or(false)
}

/// The draft kept by `save_draft`, if any.
pub async fn load_draft() -> Option<String> {
    let script = format!(
        "try {{ {OPEN_DB}
const value = await new Promise((ok) => {{
  const r = db.transaction('drafts').objectStore('drafts').get('current');
  r.onsuccess = () => ok(r.result || ''); r.onerror = () => ok('');
}});
return value; }} catch (e) {{ return ''; }}"
    );
    let value: String = document::eval(&script).join().await.ok()?;
    (!value.is_empty()).then_some(value)
}

/// Asks the browser's yes/no question.
pub async fn confirm(message: &str) -> bool {
    let eval = document::eval("const m = await dioxus.recv(); return window.confirm(m);");
    if eval.send(message.to_string()).is_err() {
        return false;
    }
    eval.join::<bool>().await.unwrap_or(false)
}

/// Prints the page; the print stylesheet shows the room sheet only.
pub fn print() {
    let _ = document::eval("window.print();");
}

/// Waits for the next frame, so a "working" message shows before a long computation.
pub async fn next_frame() {
    let _ = document::eval("await new Promise((r) => setTimeout(r, 30)); return 0;")
        .join::<u32>()
        .await;
}
