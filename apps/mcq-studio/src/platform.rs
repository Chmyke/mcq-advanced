//! What the Studio needs from its host beyond the player's helpers: dropped and pasted files, the
//! microphone, the draft kept in the browser, confirmation and printing.
//!
//! Data goes through the eval channel (`dioxus.send` / `dioxus.recv`), never spliced into
//! scripts, so nothing typed or imported can be executed.

use dioxus::prelude::*;

pub use mcq_player::platform::{browser_language, download, remember_dark, remembered_dark, rng};

/// The name sent with text pasted outside any field, which may hold a paper. No file name holds
/// a NUL character.
pub const PASTED_TEXT: &str = "\u{0}text";

/// Sends `[file name, base 64]` for every file dropped on the page or pasted into it, and
/// `[PASTED_TEXT, text]` for text pasted outside a field.
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
  if (files.length) {
    e.preventDefault();
    for (const f of files) send(f);
    return;
  }
  if (e.target.closest && e.target.closest('input, textarea, select, [contenteditable]')) return;
  const text = e.clipboardData ? e.clipboardData.getData('text/plain') : '';
  if (!text) return;
  e.preventDefault();
  dioxus.send(['\u0000text', text]);
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

/// Where the draft is kept in the browser's database.
const DRAFT_KEY: &str = "current";
/// Where the teacher's identity file is kept, still locked by its passphrase.
const IDENTITY_KEY: &str = "identity";
/// Where the last published package is kept, with its file name and title.
const PUBLISHED_KEY: &str = "published";
/// Where the teacher's draft waits while the sample class's paper is in the editor.
const ASIDE_KEY: &str = "aside";

/// Keeps `value` (base 64) under `key` in the browser's database: no size limit to speak of,
/// unlike `localStorage`. An empty value removes the key.
async fn store_put(key: &str, value: String) -> bool {
    let script = format!(
        "{OPEN_DB}
const [key, data] = await dioxus.recv();
await new Promise((ok, ko) => {{
  const t = db.transaction('drafts', 'readwrite');
  if (data) t.objectStore('drafts').put(data, key); else t.objectStore('drafts').delete(key);
  t.oncomplete = ok; t.onerror = () => ko(t.error);
}});
return true;"
    );
    let wrapped = format!("try {{ {script} }} catch (e) {{ return false; }}");
    let eval = document::eval(&wrapped);
    if eval.send((key.to_string(), value)).is_err() {
        return false;
    }
    eval.join::<bool>().await.unwrap_or(false)
}

/// The value kept under `key` by `store_put`, if any.
async fn store_get(key: &str) -> Option<String> {
    let script = format!(
        "try {{ {OPEN_DB}
const key = await dioxus.recv();
const value = await new Promise((ok) => {{
  const r = db.transaction('drafts').objectStore('drafts').get(key);
  r.onsuccess = () => ok(r.result || ''); r.onerror = () => ok('');
}});
return value; }} catch (e) {{ return ''; }}"
    );
    let eval = document::eval(&script);
    eval.send(key.to_string()).ok()?;
    let value: String = eval.join().await.ok()?;
    (!value.is_empty()).then_some(value)
}

/// Keeps the draft (a `.qcmz` in base 64) in the browser.
pub async fn save_draft(base64: String) -> bool {
    store_put(DRAFT_KEY, base64).await
}

/// The draft kept by `save_draft`, if any.
pub async fn load_draft() -> Option<String> {
    store_get(DRAFT_KEY).await
}

/// Keeps the teacher's identity file (base 64) in the browser. It stays locked by its
/// passphrase: whoever opens this browser still needs the passphrase to use it.
pub async fn keep_identity(base64: String) -> bool {
    store_put(IDENTITY_KEY, base64).await
}

/// The identity file kept by `keep_identity`, if any.
pub async fn kept_identity() -> Option<String> {
    store_get(IDENTITY_KEY).await
}

/// Forgets the identity kept in this browser.
pub async fn forget_identity() -> bool {
    store_put(IDENTITY_KEY, String::new()).await
}

/// Keeps the last paper published from this browser (`KeptPaper::encode`), for Correct.
pub async fn keep_published(text: String) -> bool {
    store_put(PUBLISHED_KEY, text).await
}

/// The paper kept by `keep_published`, if any.
pub async fn kept_published() -> Option<String> {
    store_get(PUBLISHED_KEY).await
}

/// Sets the draft (a `.qcmz` in base 64) aside during the sample class; an empty value clears
/// it.
pub async fn set_aside_draft(base64: String) -> bool {
    store_put(ASIDE_KEY, base64).await
}

/// The draft set aside by `set_aside_draft`, if any: a sample class visit is under way.
pub async fn aside_draft() -> Option<String> {
    store_get(ASIDE_KEY).await
}

/// Removes `?demo=teacher` from the address, so that a reload after the visit keeps the draft.
pub fn forget_demo_link() {
    let _ = document::eval(
        "const q = new URLSearchParams(location.search); q.delete('demo'); const s = q.toString();
history.replaceState(null, '', location.pathname + (s ? '?' + s : '') + location.hash);",
    );
}

/// The paper named by the page's `?open=` link, as `(file name, bytes)`: `Some(None)` when the
/// link names one that cannot be loaded, `None` without a link. Only papers of this site open
/// (see `mcq_player::link::is_site_path`); the link is then removed from the address, so a reload
/// keeps the draft instead of importing the paper again.
pub async fn paper_from_link() -> Option<Option<(String, Vec<u8>)>> {
    const SCRIPT: &str = r#"
const q = new URLSearchParams(location.search);
const s = q.get('open');
if (!s) return '';
q.delete('open');
const rest = q.toString();
history.replaceState(null, '', location.pathname + (rest ? '?' + rest : '') + location.hash);
try { const u = new URL(s, location.href); return u.origin === location.origin ? u.pathname : '-'; }
catch (e) { return '-'; }
"#;
    const FETCH: &str = r#"
const p = await dioxus.recv();
try {
  const r = await fetch(p);
  if (!r.ok) return '';
  const b = new Uint8Array(await r.arrayBuffer());
  let s = '';
  for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000));
  return btoa(s);
} catch (e) { return ''; }
"#;
    let path: String = document::eval(SCRIPT).join().await.ok()?;
    if path.is_empty() {
        return None;
    }
    if !mcq_player::link::is_site_path(&path) {
        return Some(None);
    }
    let eval = document::eval(FETCH);
    if eval.send(path.clone()).is_err() {
        return Some(None);
    }
    let text: String = eval.join().await.unwrap_or_default();
    let name = path.rsplit('/').next().unwrap_or_default().to_string();
    Some(
        mcq_player::b64::decode(&text)
            .filter(|b| !b.is_empty())
            .map(|b| (name, b)),
    )
}

/// The interface language: the link's `lang=` (as the website's demo links give it), otherwise
/// the browser's.
pub async fn interface_language() -> String {
    const SCRIPT: &str = "return new URLSearchParams(location.search).get('lang') || '';";
    let tag: String = document::eval(SCRIPT).join().await.unwrap_or_default();
    if tag.is_empty() {
        browser_language().await
    } else {
        tag
    }
}

/// True when the page's link asks for the teacher demo (`?demo=teacher`). The link stays in the
/// address: a reload opens the demo again, over a draft that already is its paper.
pub async fn teacher_demo_from_link() -> bool {
    const SCRIPT: &str = "return new URLSearchParams(location.search).get('demo') === 'teacher';";
    document::eval(SCRIPT).join().await.unwrap_or(false)
}

/// The text on the clipboard, or `None` when the browser refuses to give it.
pub async fn read_clipboard() -> Option<String> {
    const SCRIPT: &str =
        "try { return await navigator.clipboard.readText(); } catch (e) { return null; }";
    document::eval(SCRIPT)
        .join::<Option<String>>()
        .await
        .ok()
        .flatten()
}

/// Puts `text` on the clipboard; false when the browser refuses. The text goes to the script
/// as a message, never spliced into it.
pub async fn copy_text(text: &str) -> bool {
    let eval = document::eval(
        "const t = await dioxus.recv(); try { await navigator.clipboard.writeText(t); return true; } catch (e) { return false; }",
    );
    if eval.send(text.to_string()).is_err() {
        return false;
    }
    eval.join::<bool>().await.unwrap_or(false)
}

/// Asks the browser's yes/no question.
pub async fn confirm(message: &str) -> bool {
    let eval = document::eval("const m = await dioxus.recv(); return window.confirm(m);");
    if eval.send(message.to_string()).is_err() {
        return false;
    }
    eval.join::<bool>().await.unwrap_or(false)
}

/// Asks a question with several answers in a dialog of the page, the first one highlighted;
/// returns the index of the answer, or `None` when the dialog is closed (Escape).
/// The texts go through the eval channel and into `textContent`: never parsed as markup.
pub async fn choose(message: &str, answers: &[&str]) -> Option<usize> {
    const SCRIPT: &str = r#"
const [m, answers] = await dioxus.recv();
const d = document.createElement('dialog');
d.className = 'choice-dialog card';
const p = document.createElement('p');
p.textContent = m;
const row = document.createElement('div');
row.className = 'nav';
d.append(p, row);
const picked = new Promise((ok) => {
  answers.forEach((text, i) => {
    const b = document.createElement('button');
    b.type = 'button';
    b.textContent = text;
    if (i === 0) b.className = 'primary';
    b.addEventListener('click', () => { ok(i); d.close(); });
    row.append(b);
  });
  d.addEventListener('close', () => ok(-1));
});
(document.querySelector('.app') || document.body).append(d);
d.showModal();
row.firstChild.focus();
const i = await picked;
d.remove();
return i;
"#;
    let eval = document::eval(SCRIPT);
    let answers: Vec<String> = answers.iter().map(|a| a.to_string()).collect();
    eval.send((message.to_string(), answers)).ok()?;
    let picked: i64 = eval.join().await.ok()?;
    usize::try_from(picked).ok()
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
