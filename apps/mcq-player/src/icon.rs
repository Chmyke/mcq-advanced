//! The interface's icons, drawn for this project on one 24-unit grid with the same rounded
//! stroke, and the buttons that show them. An icon never stands alone: its button carries a
//! name for screen readers and a tip that appears on hover and keyboard focus, and under the
//! icon on touch screens, where nothing hovers.

use dioxus::prelude::*;

/// Every icon of both programs. The stroke, its width and its colour come from the stylesheet
/// (`.glyph`), so one rule keeps them consistent and they follow the light and dark themes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Icon {
    ArrowDown,
    ArrowLeft,
    ArrowRight,
    ArrowUp,
    Braces,
    Clock,
    Close,
    Copy,
    Correct,
    Download,
    FileNew,
    Flag,
    Image,
    Keyboard,
    Mic,
    Monitor,
    Moon,
    Open,
    Paste,
    Pen,
    Phone,
    Plus,
    Print,
    Send,
    Share,
    Sheet,
    Sigma,
    Sliders,
    Stop,
    Sun,
    Text,
    TextLarger,
    TextSmaller,
    Trash,
    Undo,
}

impl Icon {
    pub const ALL: [Icon; 35] = [
        Icon::ArrowDown,
        Icon::ArrowLeft,
        Icon::ArrowRight,
        Icon::ArrowUp,
        Icon::Braces,
        Icon::Clock,
        Icon::Close,
        Icon::Copy,
        Icon::Correct,
        Icon::Download,
        Icon::FileNew,
        Icon::Flag,
        Icon::Image,
        Icon::Keyboard,
        Icon::Mic,
        Icon::Monitor,
        Icon::Moon,
        Icon::Open,
        Icon::Paste,
        Icon::Pen,
        Icon::Phone,
        Icon::Plus,
        Icon::Print,
        Icon::Send,
        Icon::Share,
        Icon::Sheet,
        Icon::Sigma,
        Icon::Sliders,
        Icon::Stop,
        Icon::Sun,
        Icon::Text,
        Icon::TextLarger,
        Icon::TextSmaller,
        Icon::Trash,
        Icon::Undo,
    ];

    /// The drawing, as one SVG path on the 24 x 24 grid. Circles are written as two arcs.
    pub fn path(self) -> &'static str {
        match self {
            Icon::ArrowDown => "M12 5v14M6 13l6 6 6-6",
            Icon::ArrowLeft => "M19 12H5M11 6l-6 6 6 6",
            Icon::ArrowRight => "M5 12h14M13 6l6 6-6 6",
            Icon::ArrowUp => "M12 19V5M6 11l6-6 6 6",
            Icon::Braces => {
                "M8 4a2 2 0 0 0-2 2v4l-2 2 2 2v4a2 2 0 0 0 2 2M16 4a2 2 0 0 1 2 2v4l2 2-2 2v4a2 2 0 0 1-2 2"
            }
            Icon::Clock => "M3 12a9 9 0 1 0 18 0a9 9 0 1 0-18 0M12 7v5l3 2",
            Icon::Close => "M6 6l12 12M18 6L6 18",
            Icon::Copy => {
                "M11 9h7a2 2 0 0 1 2 2v7a2 2 0 0 1-2 2h-7a2 2 0 0 1-2-2v-7a2 2 0 0 1 2-2zM5 15a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1h9a1 1 0 0 1 1 1"
            }
            Icon::Correct => {
                "M9 3h6v3H9zM15 4.5h2a2 2 0 0 1 2 2V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6.5a2 2 0 0 1 2-2h2M9 14l2 2 4-4"
            }
            Icon::Download => "M12 4v11M7 10l5 5 5-5M5 20h14",
            Icon::FileNew => {
                "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M12 11v6M9 14h6"
            }
            Icon::Flag => "M5 21V4M5 4h12l-2.5 4.5L17 13H5",
            Icon::Image => {
                "M5 4h14a2 2 0 0 1 2 2v12a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2V6a2 2 0 0 1 2-2zM7.5 9a1.5 1.5 0 1 0 3 0a1.5 1.5 0 1 0-3 0M21 15l-5-5L5 20"
            }
            Icon::Keyboard => {
                "M4 6h16a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H4a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2zM6 10h.01M10 10h.01M14 10h.01M18 10h.01M8 14h8"
            }
            Icon::Mic => {
                "M12 3a3 3 0 0 0-3 3v6a3 3 0 0 0 6 0V6a3 3 0 0 0-3-3zM5 11a7 7 0 0 0 14 0M12 18v3M9 21h6"
            }
            Icon::Monitor => {
                "M4 4h16a1 1 0 0 1 1 1v10a1 1 0 0 1-1 1H4a1 1 0 0 1-1-1V5a1 1 0 0 1 1-1zM8 20h8M12 16v4"
            }
            Icon::Moon => "M20 14.5A8 8 0 1 1 9.5 4a6.5 6.5 0 0 0 10.5 10.5z",
            Icon::Open => {
                "M3 6a2 2 0 0 1 2-2h4l2 2h8a2 2 0 0 1 2 2v10a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2zM12 17v-6M9 14l3-3 3 3"
            }
            Icon::Paste => {
                "M9 3h6v3H9zM15 4.5h2a2 2 0 0 1 2 2V19a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6.5a2 2 0 0 1 2-2h2M12 10v7M9 14l3 3 3-3"
            }
            Icon::Pen => "M4 20h4L19 9a2.83 2.83 0 0 0-4-4L4 16zM13.5 6.5l4 4",
            Icon::Phone => {
                "M8 2h8a2 2 0 0 1 2 2v16a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V4a2 2 0 0 1 2-2zM11 18h2"
            }
            Icon::Plus => "M12 5v14M5 12h14",
            Icon::Print => {
                "M6 9V3h12v6M6 18H4a2 2 0 0 1-2-2v-5a2 2 0 0 1 2-2h16a2 2 0 0 1 2 2v5a2 2 0 0 1-2 2h-2M6 14h12v7H6z"
            }
            Icon::Send => "M21 3L10 14M21 3l-7 18-4-7-7-4z",
            Icon::Share => {
                "M15.5 5a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0-5 0M3.5 12a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0-5 0M15.5 19a2.5 2.5 0 1 0 5 0a2.5 2.5 0 1 0-5 0M8.2 10.8l7.6-4.4M8.2 13.2l7.6 4.4"
            }
            Icon::Sheet => {
                "M14 3H7a2 2 0 0 0-2 2v14a2 2 0 0 0 2 2h10a2 2 0 0 0 2-2V8zM14 3v5h5M9 13h6M9 17h6"
            }
            Icon::Sigma => "M18 7V4H6l6 8-6 8h12v-3",
            Icon::Sliders => {
                "M4 6h10M18 6h2M4 12h4M12 12h8M4 18h12M20 18h.01M14 6a2 2 0 1 0 4 0a2 2 0 1 0-4 0M8 12a2 2 0 1 0 4 0a2 2 0 1 0-4 0M16 18a2 2 0 1 0 4 0a2 2 0 1 0-4 0"
            }
            Icon::Stop => "M8 6h8a2 2 0 0 1 2 2v8a2 2 0 0 1-2 2H8a2 2 0 0 1-2-2V8a2 2 0 0 1 2-2z",
            Icon::Sun => {
                "M8 12a4 4 0 1 0 8 0a4 4 0 1 0-8 0M12 2v2M12 20v2M4.9 4.9l1.4 1.4M17.7 17.7l1.4 1.4M2 12h2M20 12h2M4.9 19.1l1.4-1.4M17.7 6.3l1.4-1.4"
            }
            Icon::Text => "M5 7V4h14v3M12 4v16M9 20h6",
            Icon::TextLarger => "M2 19l5-14 5 14M4 14h6M15 12h7M18.5 8.5v7",
            Icon::TextSmaller => "M2 19l5-14 5 14M4 14h6M15 12h7",
            Icon::Trash => {
                "M4 7h16M10 11v6M14 11v6M6 7l1 12a2 2 0 0 0 2 2h6a2 2 0 0 0 2-2l1-12M9 7V4h6v3"
            }
            Icon::Undo => "M9 14L4 9l5-5M4 9h10.5a5.5 5.5 0 0 1 0 11H11",
        }
    }
}

/// Keeps tips and menus inside the window. Where a button sits depends on the window's width
/// (the Studio's bar wraps), so no fixed side fits: each tip or menu is measured as it shows
/// and nudged back in. Installed once per page.
const KEEP_IN_VIEW_SCRIPT: &str = r#"
if (!window.mcqKeepInView) {
  window.mcqKeepInView = true;
  const fit = (el) => {
    el.style.translate = '';
    const r = el.getBoundingClientRect();
    const dx = r.left < 4 ? 4 - r.left : r.right > innerWidth - 4 ? innerWidth - 4 - r.right : 0;
    if (dx) el.style.translate = dx + 'px 0';
  };
  const around = (e) => {
    const host = e.target.closest && e.target.closest('.icon-menu, [data-icon]');
    if (!host) return;
    // Two frames: a menu opened by this click is drawn after the event.
    requestAnimationFrame(() => requestAnimationFrame(() =>
      host.querySelectorAll('.tip, .menu-list').forEach(fit)));
  };
  for (const type of ['mouseover', 'focusin', 'click']) document.addEventListener(type, around, true);
}
"#;

/// Installs the script that keeps tips and menus inside the window; call it from the root.
pub fn use_keep_in_view() {
    use_effect(|| {
        document::eval(KEEP_IN_VIEW_SCRIPT);
    });
}

/// What breaks the icon contract in rendered HTML, one line per fault: every icon element has
/// a non-empty accessible name, its tip says the same, and the name contains the short label a
/// touch screen shows (WCAG 2.5.3, label in name). The tests of both programs run it on their
/// screens, so a button added by hand without a name fails them.
pub fn contract_faults(html: &str) -> Vec<String> {
    let mut faults = Vec::new();
    let mut from = 0;
    while let Some(found) = html[from..].find("data-icon=\"") {
        let at = from + found;
        from = at + 1;
        if html[at..].starts_with("data-icon=\"text\"") {
            continue;
        }
        let start = html[..at].rfind('<').unwrap_or(0);
        let tag: String = html[start + 1..]
            .chars()
            .take_while(|c| c.is_ascii_alphanumeric())
            .collect();
        let end = html[start..]
            .find(&format!("</{tag}>"))
            .map_or(html.len(), |e| start + e);
        let element = &html[start..end];
        let name = attribute(element, "aria-label").unwrap_or_default();
        let label = span_text(element, "label").unwrap_or_default();
        let tip = span_text(element, "tip").unwrap_or_default();
        if name.trim().is_empty() {
            faults.push(format!("no accessible name: {element}"));
        } else if !name.to_lowercase().contains(&label.to_lowercase()) {
            faults.push(format!(
                "the name \"{name}\" does not contain the label \"{label}\""
            ));
        }
        if label.trim().is_empty() {
            faults.push(format!("no label: {element}"));
        }
        if tip != name {
            faults.push(format!(
                "the tip \"{tip}\" differs from the name \"{name}\""
            ));
        }
    }
    faults
}

/// The first value of `name="..."` in `html`.
fn attribute<'a>(html: &'a str, name: &str) -> Option<&'a str> {
    let key = format!(" {name}=\"");
    let start = html.find(&key)? + key.len();
    Some(&html[start..start + html[start..].find('"')?])
}

/// The text of the first `<span class="{class}" ...>`.
fn span_text<'a>(html: &'a str, class: &str) -> Option<&'a str> {
    let open = html.find(&format!("<span class=\"{class}\""))?;
    let start = open + html[open..].find('>')? + 1;
    Some(&html[start..start + html[start..].find("</span>")?])
}

/// The drawing alone, hidden from screen readers: the element around it carries the name.
#[component]
pub fn Glyph(icon: Icon) -> Element {
    rsx! {
        svg { class: "glyph", "viewBox": "0 0 24 24", "aria-hidden": "true",
            path { d: icon.path() }
        }
    }
}

/// A button shown as an icon. `label` is its short name, written under the icon on touch
/// screens and next to it when `shown`; `tip` explains the action on hover and keyboard focus
/// and is the name screen readers announce, so it must contain the label (WCAG 2.5.3).
/// `shown` is for main navigation and decisive actions: the label then stays next to the icon,
/// except on narrow screens.
#[component]
pub fn IconButton(
    icon: Icon,
    label: String,
    tip: Option<String>,
    #[props(default)] shown: bool,
    onclick: Option<EventHandler<MouseEvent>>,
    #[props(extends = GlobalAttributes, extends = button)] attributes: Vec<Attribute>,
) -> Element {
    let tip = tip.unwrap_or_else(|| label.clone());
    rsx! {
        button {
            "data-icon": if shown { "shown" } else { "tip" },
            "aria-label": "{tip}",
            onclick: move |e| {
                if let Some(handler) = onclick {
                    handler.call(e);
                }
            },
            ..attributes,
            Glyph { icon }
            span { class: "label", "{label}" }
            span { class: "tip", "aria-hidden": "true", "{tip}" }
        }
    }
}

/// An icon that opens a short list of actions written out in words, for actions that would
/// otherwise look alike (four exports, three downloads). Choosing one or pressing Escape
/// closes the list.
#[component]
pub fn IconMenu(icon: Icon, label: String, tip: Option<String>, children: Element) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        div { class: "icon-menu",
            onkeydown: move |e: KeyboardEvent| {
                if e.key() == Key::Escape {
                    open.set(false);
                }
            },
            IconButton { icon, label, tip, aria_expanded: if open() { "true" } else { "false" }, onclick: move |_| open.toggle() }
            if open() {
                div { class: "menu-list", onclick: move |_| open.set(false), {children} }
            }
        }
    }
}

/// A file picker shown as an icon: a label around a hidden input, named like an `IconButton`.
#[component]
pub fn IconFile(
    icon: Icon,
    label: String,
    tip: Option<String>,
    accept: String,
    #[props(default)] multiple: bool,
    onchange: EventHandler<FormEvent>,
) -> Element {
    let tip = tip.unwrap_or_else(|| label.clone());
    rsx! {
        label { class: "file", "data-icon": "tip",
            Glyph { icon }
            span { class: "label", "aria-hidden": "true", "{label}" }
            span { class: "tip", "aria-hidden": "true", "{tip}" }
            input { r#type: "file", class: "sr-only", "aria-label": "{tip}", accept: "{accept}", multiple,
                onchange: move |e| onchange.call(e),
            }
        }
    }
}
