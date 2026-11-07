//! The first screen over a blank draft: the four ways to start, so a teacher who opens the Studio
//! for the first time is not left in front of an empty question and its list of errors.

use dioxus::prelude::*;
use mcq_player::icon::{Icon, IconButton, IconFile};

use crate::actions;
use crate::correction::Correction;
use crate::platform;
use crate::state::Studio;
use crate::texts::{Lang, Texts};

/// The prompt that makes an AI write a paper in the Studio's format, as the website offers it.
pub fn ai_prompt(lang: Lang) -> &'static str {
    match lang {
        Lang::Fr => include_str!("../../../examples/ai-prompt/fr.txt"),
        Lang::En => include_str!("../../../examples/ai-prompt/en.txt"),
    }
}

/// Copies the AI prompt and says so; offers it as a file when the clipboard is refused.
pub async fn copy_ai_prompt(mut notice: Signal<Option<String>>, t: &'static Texts) {
    let prompt = ai_prompt(t.lang);
    if platform::copy_text(prompt).await {
        notice.set(Some(t.prompt_copied.to_string()));
    } else {
        platform::download(t.prompt_file, &mcq_player::b64::encode(prompt.as_bytes()));
        notice.set(Some(t.prompt_saved.to_string()));
    }
}

#[component]
pub fn Welcome(
    studio: Signal<Studio>,
    correction: Signal<Correction>,
    notice: Signal<Option<String>>,
    tour: Signal<bool>,
    welcome: Signal<bool>,
    lang: Lang,
    t: &'static Texts,
) -> Element {
    let mut welcome = welcome;
    rsx! {
        section { class: "card welcome-studio", "aria-labelledby": "welcome-title",
            h1 { id: "welcome-title", "{t.welcome_title}" }
            p { "{t.welcome_text}" }
            div { class: "welcome-ways",
                div { class: "way",
                    IconButton { class: "primary", icon: Icon::Pen, label: t.welcome_write, shown: true,
                        onclick: move |_| {
                            welcome.set(false);
                            let _ = document::eval("setTimeout(() => { const b = document.querySelector('.statement textarea'); if (b) b.focus(); }, 0);");
                        },
                    }
                    p { class: "muted", "{t.welcome_write_hint}" }
                }
                div { class: "way",
                    IconFile { icon: Icon::Open, label: t.import, tip: t.tip_import, accept: actions::PAPER_FILES, shown: true,
                        onchange: move |e: FormEvent| async move {
                            if let Some(file) = e.files().into_iter().next()
                                && let Ok(bytes) = file.read_bytes().await
                            {
                                actions::open_paper(studio, notice, t, &file.name(), bytes.to_vec()).await;
                            }
                        },
                    }
                    p { class: "muted", "{t.welcome_import_hint}" }
                }
                div { class: "way",
                    IconButton { icon: Icon::Copy, label: t.copy_prompt, shown: true,
                        onclick: move |_| async move { copy_ai_prompt(notice, t).await },
                    }
                    p { class: "muted", "{t.welcome_ai_hint}" }
                }
                div { class: "way",
                    IconButton { icon: Icon::Correct, label: t.welcome_demo, shown: true,
                        onclick: move |_| async move {
                            welcome.set(false);
                            crate::demo::open(studio, correction, notice, tour, t, lang).await;
                        },
                    }
                    p { class: "muted", "{t.welcome_demo_hint}" }
                }
            }
        }
    }
}
