//! Starts the Studio in the browser (`--features web`) or in a desktop window (`--features desktop`).

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, WindowBuilder};
        let window = WindowBuilder::new().with_title("MCQ Advanced Studio");
        // Files dropped on the window reach the page, which adds them to the paper.
        let config = Config::new()
            .with_window(window)
            .with_menu(None)
            .with_disable_drag_drop_handler(true);
        dioxus::LaunchBuilder::desktop()
            .with_cfg(config)
            .launch(mcq_studio::App);
    }
    #[cfg(all(feature = "web", not(feature = "desktop")))]
    dioxus::launch(mcq_studio::App);
    #[cfg(not(any(feature = "web", feature = "desktop")))]
    eprintln!("Build the Studio with `--features web` or `--features desktop`.");
}
