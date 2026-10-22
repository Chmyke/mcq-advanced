//! Gives the Windows program its icon and its name, as Explorer, the taskbar and the shortcuts
//! show them. Resource 32512 is the one the desktop window takes its own icon from.

fn main() {
    let icon = "../../packaging/icons/mcq-advanced.ico";
    println!("cargo:rerun-if-changed={icon}");
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let mut resource = winresource::WindowsResource::new();
    resource
        .set_icon_with_id(icon, "32512")
        .set("ProductName", "MCQ Advanced")
        .set("FileDescription", "MCQ Advanced Studio");
    if let Err(error) = resource.compile() {
        panic!("Cannot embed the Windows icon: {error}");
    }
}
