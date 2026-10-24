//! What the player needs from its host, through the page's JavaScript so the same code runs in
//! the browser and in the desktop webview: storage, downloads, language, events.
//!
//! Only hex, base 64 and plain ASCII names are ever spliced into scripts, so nothing a student
//! types or a paper contains can be executed.

use dioxus::prelude::*;
use mcq_crypto::rand_core::UnwrapErr;

pub fn rng() -> UnwrapErr<getrandom::SysRng> {
    UnwrapErr(getrandom::SysRng)
}

pub fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

/// A storage key per paper and student, from hex only.
pub fn save_key(paper_id: &[u8; 16], student_name: &str) -> String {
    format!(
        "mcq-session-{}-{}",
        hex(paper_id),
        hex(student_name.to_uppercase().as_bytes())
    )
}

fn script_safe(text: &str) -> bool {
    text.bytes()
        .all(|b| b.is_ascii_alphanumeric() || b"+/=-_.".contains(&b))
}

pub async fn storage_get(key: &str) -> Option<String> {
    if !script_safe(key) {
        return None;
    }
    let script =
        format!("try {{ return localStorage.getItem('{key}') || ''; }} catch (e) {{ return ''; }}");
    let value: String = document::eval(&script).join().await.ok()?;
    (!value.is_empty()).then_some(value)
}

pub fn storage_set(key: &str, value: &str) {
    if !script_safe(key) || !script_safe(value) {
        return;
    }
    let _ = document::eval(&format!(
        "try {{ localStorage.setItem('{key}', '{value}'); }} catch (e) {{}}"
    ));
}

/// Offers a file to save; in the desktop webview it lands in the Downloads folder.
pub fn download(file_name: &str, base64: &str) {
    let name: String = file_name
        .chars()
        .map(|c| {
            if c.is_ascii_alphanumeric() || c == '.' || c == '-' {
                c
            } else {
                '_'
            }
        })
        .collect();
    if !script_safe(base64) {
        return;
    }
    let _ = document::eval(&format!(
        "const a = document.createElement('a'); \
         a.href = 'data:application/octet-stream;base64,{base64}'; \
         a.download = '{name}'; document.body.appendChild(a); a.click(); a.remove();"
    ));
}

/// True when the system asks for a dark theme.
pub async fn prefers_dark() -> bool {
    document::eval("return window.matchMedia('(prefers-color-scheme: dark)').matches;")
        .join()
        .await
        .unwrap_or(false)
}

pub async fn browser_language() -> String {
    document::eval("return navigator.language || 'en';")
        .join()
        .await
        .unwrap_or_else(|_| "en".to_string())
}

/// The package named by the page's `?paper=` link, fetched and returned in base 64, or nothing.
/// The script reads the link itself: nothing from it is spliced into code.
pub async fn package_from_link() -> Option<Vec<u8>> {
    const SCRIPT: &str = r#"
const p = new URLSearchParams(location.search).get('paper');
if (!p) return '';
try {
  const r = await fetch(p);
  if (!r.ok) return '';
  const b = new Uint8Array(await r.arrayBuffer());
  let s = '';
  for (let i = 0; i < b.length; i += 0x8000) s += String.fromCharCode.apply(null, b.subarray(i, i + 0x8000));
  return btoa(s);
} catch (e) { return ''; }
"#;
    let text: String = document::eval(SCRIPT).join().await.ok()?;
    if text.is_empty() {
        return None;
    }
    crate::b64::decode(&text)
}

/// The other options of the page's link (`name`, `lang`, `source`); `source` comes back as an
/// absolute path, and only when it is on this site.
pub async fn link_options() -> crate::link::LinkOptions {
    const SCRIPT: &str = r#"
const q = new URLSearchParams(location.search);
let source = '';
try {
  const s = q.get('source');
  if (s) { const u = new URL(s, location.href); if (u.origin === location.origin) source = u.pathname; }
} catch (e) {}
return [q.get('name') || '', q.get('lang') || '', source];
"#;
    let [name, lang, source]: [String; 3] = document::eval(SCRIPT).join().await.unwrap_or_default();
    crate::link::LinkOptions { name, lang, source }
}

/// Sends `tick` every second, `away` and `back` on focus changes, and `k:<key>` on key presses
/// outside text fields. Single-character keys are dropped while the student turned shortcuts off.
pub const EVENTS_SCRIPT: &str = r#"
setInterval(() => dioxus.send('tick'), 1000);
let away = false;
const leave = () => { if (!away) { away = true; dioxus.send('away'); } };
const come = () => { if (away) { away = false; dioxus.send('back'); } };
document.addEventListener('visibilitychange', () => document.hidden ? leave() : come());
window.addEventListener('blur', leave);
window.addEventListener('focus', come);
document.addEventListener('keydown', (e) => {
  const t = e.target;
  if (t && (t.tagName === 'TEXTAREA' || (t.tagName === 'INPUT' && (t.type === 'text' || t.type === 'password')))) return;
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  const app = document.querySelector('.app');
  if (e.key.length === 1 && app && app.dataset.shortcuts === 'off') return;
  // A focused button or box answers Space and Enter itself.
  if ((e.key === ' ' || e.key === 'Enter') && t && ['BUTTON', 'INPUT', 'SELECT', 'A', 'SUMMARY'].includes(t.tagName)) return;
  if (e.key === ' ') e.preventDefault();
  if ((e.key === 'ArrowLeft' || e.key === 'ArrowRight') && t && t.type === 'radio') e.preventDefault();
  dioxus.send('k:' + e.key);
});
await new Promise(() => {});
"#;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_hold_hex_only() {
        let key = save_key(&[0xAB; 16], "Zoë O'Brien");
        assert!(script_safe(&key), "{key}");
        assert!(!script_safe("x'); alert(1); ('"));
    }
}
