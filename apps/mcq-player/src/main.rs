//! Starts the player in the browser (`--features web`) or in a desktop window (`--features desktop`).

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, WindowBuilder};
        let window = WindowBuilder::new().with_title("MCQ Advanced");
        let config = Config::new().with_window(window).with_menu(None);
        // On Windows the window takes its icon from the program's resource (build.rs).
        #[cfg(not(windows))]
        let config = match dioxus::desktop::icon_from_memory::<dioxus::desktop::tao::window::Icon>(
            include_bytes!("../../../packaging/icons/mcq-advanced-256.png"),
        ) {
            Ok(icon) => config.with_icon(icon),
            Err(_) => config,
        };
        dioxus::LaunchBuilder::desktop()
            .with_cfg(config)
            .launch(mcq_player::App);
    }
    #[cfg(all(feature = "web", not(feature = "desktop")))]
    dioxus::launch(mcq_player::App);
    #[cfg(not(any(feature = "web", feature = "desktop")))]
    eprintln!("Build the player with `--features web` or `--features desktop`.");
}
