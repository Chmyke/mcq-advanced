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

/// Writes the current time, in seconds, next to `key`: when the page closes, the last stamp says
/// when the exam stopped counting.
pub fn storage_stamp(key: &str) {
    if !script_safe(key) {
        return;
    }
    let _ = document::eval(&format!(
        "try {{ localStorage.setItem('{key}-at', String(Math.floor(Date.now() / 1000))); }} catch (e) {{}}"
    ));
}

/// Seconds since the last `storage_stamp` of `key`, 0 without one or with a clock gone back.
pub async fn seconds_since_stamp(key: &str) -> u32 {
    if !script_safe(key) {
        return 0;
    }
    let script = format!(
        "try {{ const v = Number(localStorage.getItem('{key}-at')); \
         return v > 0 ? Math.max(0, Math.floor(Date.now() / 1000) - v) : 0; }} catch (e) {{ return 0; }}"
    );
    let seconds: f64 = document::eval(&script).join().await.unwrap_or(0.0);
    // WHY: a value from the page, clamped before the cast.
    seconds.clamp(0.0, f64::from(u32::MAX)) as u32
}

/// Removes `key` and its stamp.
pub fn storage_remove(key: &str) {
    if !script_safe(key) {
        return;
    }
    let _ = document::eval(&format!(
        "try {{ localStorage.removeItem('{key}'); localStorage.removeItem('{key}-at'); }} catch (e) {{}}"
    ));
}

/// The theme picked on this device, shared by the player and the Studio.
const THEME_KEY: &str = "mcq-theme";

/// Dark unless light was picked on this device.
pub async fn remembered_dark() -> bool {
    stored_dark(storage_get(THEME_KEY).await.as_deref())
}

pub fn remember_dark(dark: bool) {
    storage_set(THEME_KEY, if dark { "dark" } else { "light" });
}

/// Anything but a stored "light", nothing stored included, means dark.
fn stored_dark(value: Option<&str>) -> bool {
    value != Some("light")
}

/// A file name safe to splice into a script: accents dropped (`Léa` gives `Lea`), anything
/// else outside letters, digits, `.` and `-` turned into `_`.
pub fn file_name(name: &str) -> String {
    name.chars()
        .map(|c| match ascii_letter(c) {
            Some(plain) => plain,
            None if c.is_ascii_alphanumeric() || c == '.' || c == '-' => c,
            None => '_',
        })
        .collect()
}

/// The plain letter under a Latin accented one, as names are written in class.
fn ascii_letter(c: char) -> Option<char> {
    const TABLE: [(&str, char); 14] = [
        ("àáâãäåāă", 'a'),
        ("ÀÁÂÃÄÅĀĂ", 'A'),
        ("çćč", 'c'),
        ("ÇĆČ", 'C'),
        ("èéêëēěę", 'e'),
        ("ÈÉÊËĒĚĘ", 'E'),
        ("ìíîïī", 'i'),
        ("ÌÍÎÏĪ", 'I'),
        ("ñń", 'n'),
        ("ÑŃ", 'N'),
        ("òóôõöøō", 'o'),
        ("ÒÓÔÕÖØŌ", 'O'),
        ("ùúûüūů", 'u'),
        ("ÙÚÛÜŪŮ", 'U'),
    ];
    match c {
        'ÿ' | 'ý' => Some('y'),
        'Ÿ' | 'Ý' => Some('Y'),
        'š' => Some('s'),
        'Š' => Some('S'),
        'ž' => Some('z'),
        'Ž' => Some('Z'),
        _ => TABLE
            .iter()
            .find(|(accented, _)| accented.contains(c))
            .map(|(_, plain)| *plain),
    }
}

/// Offers a file to save: the browser downloads it, the desktop program asks where to put it.
pub fn download(file_name: &str, base64: &str) {
    let name = self::file_name(file_name);
    #[cfg(feature = "desktop")]
    {
        save_as(&name, base64);
    }
    #[cfg(not(feature = "desktop"))]
    {
        browser_download(&name, base64);
    }
}

/// Asks the system's "Save as" dialog where to write the file, starting from the folder used
/// last; nothing is written when the dialog is cancelled. The webview's own download would land
/// in the Downloads folder without a word.
#[cfg(feature = "desktop")]
fn save_as(name: &str, base64: &str) {
    let Some(bytes) = crate::b64::decode(base64) else {
        return;
    };
    let window = dioxus::desktop::window();
    let Some(path) = rfd::FileDialog::new()
        .set_parent(&*window.window)
        .set_file_name(name)
        .save_file()
    else {
        return;
    };
    if std::fs::write(&path, bytes).is_err() {
        let _ = document::eval("window.alert('The file could not be written there.');");
    }
}

/// Makes the browser download the file.
#[cfg(not(feature = "desktop"))]
fn browser_download(name: &str, base64: &str) {
    if !script_safe(base64) {
        return;
    }
    let _ = document::eval(&format!(
        "const a = document.createElement('a'); \
         a.href = 'data:application/octet-stream;base64,{base64}'; \
         a.download = '{name}'; document.body.appendChild(a); a.click(); a.remove();"
    ));
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

/// What a share did.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Shared {
    /// The system's share sheet took the link.
    Sent,
    /// The student closed the share sheet.
    Cancelled,
    Copied,
    Failed,
}

/// Shares the page's address without the student's name: the system's share sheet where there
/// is one, the clipboard otherwise. The script reads the address itself: nothing is spliced in.
pub async fn share_link() -> Shared {
    const SCRIPT: &str = r#"
const u = new URL(location.href);
u.searchParams.delete('name');
u.hash = '';
const url = u.toString();
if (navigator.share) {
  try { await navigator.share({ title: document.title, url }); return 'sent'; }
  catch (e) { if (e && e.name === 'AbortError') return 'cancelled'; }
}
try { await navigator.clipboard.writeText(url); return 'copied'; } catch (e) { return ''; }
"#;
    let outcome: String = document::eval(SCRIPT).join().await.unwrap_or_default();
    match outcome.as_str() {
        "sent" => Shared::Sent,
        "cancelled" => Shared::Cancelled,
        "copied" => Shared::Copied,
        _ => Shared::Failed,
    }
}

/// Where the unfinished copy of this device is noted, for the welcome screen.
const IN_PROGRESS_KEY: &str = "mcq-in-progress";

/// Notes that the copy saved under `key` is unfinished, with its paper's title.
pub fn remember_in_progress(key: &str, title: &str) {
    if script_safe(key) {
        storage_set(
            IN_PROGRESS_KEY,
            &format!("{key}.{}", crate::b64::encode(title.as_bytes())),
        );
    }
}

pub fn forget_in_progress() {
    storage_remove(IN_PROGRESS_KEY);
}

/// The storage key and the paper title of the unfinished copy noted on this device.
pub async fn in_progress() -> Option<(String, String)> {
    let value = storage_get(IN_PROGRESS_KEY).await?;
    let (key, title) = value.split_once('.')?;
    let title = String::from_utf8(crate::b64::decode(title)?).ok()?;
    Some((key.to_string(), title))
}

/// The saved sessions of one paper on this device, whatever the name they were taken under, as
/// stored (base 64). The prefix is hex only.
pub async fn saved_sessions(paper_id: &[u8; 16]) -> Vec<String> {
    let prefix = format!("mcq-session-{}-", hex(paper_id));
    let script = format!(
        "try {{ const out = []; for (let i = 0; i < localStorage.length; i++) {{ \
         const k = localStorage.key(i); \
         if (k && k.startsWith('{prefix}') && !k.endsWith('-at')) out.push(localStorage.getItem(k) || ''); }} \
         return out; }} catch (e) {{ return []; }}"
    );
    document::eval(&script).join().await.unwrap_or_default()
}

/// True where the system's share sheet takes files (phones, mostly).
pub async fn can_share_files() -> bool {
    const SCRIPT: &str = r#"
try { return !!(navigator.canShare && navigator.canShare({ files: [new File(['x'], 'copy.mcqres', { type: 'application/octet-stream' })] })); }
catch (e) { return false; }
"#;
    document::eval(SCRIPT).join().await.unwrap_or(false)
}

/// Hands a file to the system's share sheet (mail, messaging, school apps). The name and the
/// content are checked like a download's before they reach the script.
pub async fn share_file(file_name: &str, base64: &str) -> Shared {
    let name = self::file_name(file_name);
    if !script_safe(base64) {
        return Shared::Failed;
    }
    let script = format!(
        "const b = Uint8Array.from(atob('{base64}'), (c) => c.charCodeAt(0)); \
         const f = new File([b], '{name}', {{ type: 'application/octet-stream' }}); \
         if (!navigator.canShare || !navigator.canShare({{ files: [f] }})) return ''; \
         try {{ await navigator.share({{ files: [f], title: '{name}' }}); return 'sent'; }} \
         catch (e) {{ return e && e.name === 'AbortError' ? 'cancelled' : ''; }}"
    );
    let outcome: String = document::eval(&script).join().await.unwrap_or_default();
    match outcome.as_str() {
        "sent" => Shared::Sent,
        "cancelled" => Shared::Cancelled,
        _ => Shared::Failed,
    }
}

/// Sends `tick:<seconds>` with the whole seconds of the wall clock gone by since the last one,
/// `away` and `back` on focus changes, and `k:<key>` on key presses outside text fields.
/// Single-character keys are dropped while the student turned shortcuts off.
/// WHY the wall clock: a browser slows or freezes the timers of a hidden page or a locked phone;
/// counting beats would then hand out free time. A clock set back counts nothing.
pub const EVENTS_SCRIPT: &str = r#"
let last = Date.now(), carry = 0;
const beat = () => {
  const now = Date.now();
  const ms = Math.max(0, now - last) + carry;
  last = now;
  const s = Math.floor(ms / 1000);
  carry = ms - s * 1000;
  if (s > 0) dioxus.send('tick:' + s);
};
setInterval(beat, 1000);
// Leaving or reloading during the exam asks first; the answers stay on the device anyway.
window.addEventListener('beforeunload', (e) => { if (document.querySelector('.exam')) { e.preventDefault(); e.returnValue = ''; } });
let away = false;
const leave = () => { if (!away) { away = true; dioxus.send('away'); } };
const come = () => { if (away) { beat(); away = false; dioxus.send('back'); } };
document.addEventListener('visibilitychange', () => document.hidden ? leave() : come());
window.addEventListener('blur', leave);
window.addEventListener('focus', come);
document.addEventListener('keydown', (e) => {
  const t = e.target;
  if (t && (t.tagName === 'TEXTAREA' || (t.tagName === 'INPUT' && (t.type === 'text' || t.type === 'password')))) return;
  if (e.ctrlKey || e.metaKey || e.altKey) return;
  const app = document.querySelector('.app');
  if (e.key.length === 1 && app && app.dataset.shortcuts === 'off') return;
  // A focused button or box answers Space itself, and Enter too except on an answer's box,
  // where Enter moves on as the keyboard help says.
  const answerBox = t && t.tagName === 'INPUT' && (t.type === 'checkbox' || t.type === 'radio') && t.closest('.options');
  if (e.key === ' ' && t && ['BUTTON', 'INPUT', 'SELECT', 'A', 'SUMMARY'].includes(t.tagName)) return;
  if (e.key === 'Enter' && t && !answerBox && ['BUTTON', 'INPUT', 'SELECT', 'A', 'SUMMARY'].includes(t.tagName)) return;
  if (e.key === 'Enter' && answerBox) e.preventDefault();
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

    #[test]
    fn file_names_keep_the_letters_of_accented_names() {
        assert_eq!(file_name("Léa Testeuse.mcqres"), "Lea_Testeuse.mcqres");
        assert_eq!(file_name("Zoë O'Brien.mcqres"), "Zoe_O_Brien.mcqres");
        assert_eq!(file_name("Ånæs Çelik"), "An_s_Celik");
        assert!(script_safe(&file_name("x'); alert(1); ('")));
    }

    #[test]
    fn the_theme_is_dark_unless_light_was_picked() {
        assert!(stored_dark(None));
        assert!(stored_dark(Some("dark")));
        assert!(stored_dark(Some("garbage")));
        assert!(!stored_dark(Some("light")));
    }
}
