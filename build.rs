//! Bakes the app icon into the executable.
//!
//! The mark is defined once, in `src/mark.rs`, which is pulled in here with
//! `#[path]` rather than copied: the `.ico` is generated from the same arithmetic
//! the tray icon is drawn with, so there is no committed binary asset that can go
//! stale, and no rasteriser in the dependency tree.

#[path = "src/mark.rs"]
mod mark;

fn main() {
    println!("cargo:rerun-if-changed=src/mark.rs");
    println!("cargo:rerun-if-changed=build.rs");

    let out = std::path::PathBuf::from(std::env::var("OUT_DIR").expect("OUT_DIR"));
    let ico = out.join("ternitor.ico");
    std::fs::write(&ico, mark::ico()).expect("write the generated icon");

    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("the icon path is not UTF-8"));
    // The version resource, so Settings > Apps and the file's Details tab have a
    // version to read instead of the installer carrying a copy that drifts.
    let version = env!("CARGO_PKG_VERSION");
    res.set("FileVersion", version);
    res.set("ProductVersion", version);
    res.set("ProductName", "Ternitor");
    res.set("FileDescription", "Hides the blank console windows Windows opens for other processes");
    res.compile().expect(
        "no resource compiler: embedding the app icon needs rc.exe from the Windows SDK",
    );
}
