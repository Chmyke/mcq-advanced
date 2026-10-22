//! Starts the player in the browser (`--features web`) or in a desktop window (`--features desktop`).

fn main() {
    #[cfg(feature = "desktop")]
    {
        use dioxus::desktop::{Config, WindowBuilder};
        let window = WindowBuilder::new().with_title("MCQ Advanced");
        dioxus::LaunchBuilder::desktop()
            .with_cfg(Config::new().with_window(window).with_menu(None))
            .launch(mcq_player::App);
    }
    #[cfg(all(feature = "web", not(feature = "desktop")))]
    dioxus::launch(mcq_player::App);
    #[cfg(not(any(feature = "web", feature = "desktop")))]
    eprintln!("Build the player with `--features web` or `--features desktop`.");
}
