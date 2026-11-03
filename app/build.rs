//! Puts Archeion's icon in the Windows executable (what File Explorer,
//! shortcuts and the taskbar show), drawn by the same code as the window's
//! icon: no image file to keep in step; and the name, description and
//! copyright its Properties show.

use std::{env, fs, path::PathBuf};

#[path = "src/ui/icon.rs"]
mod icon;

fn main() {
    println!("cargo::rerun-if-changed=src/ui/icon.rs");
    if env::var("CARGO_CFG_TARGET_OS").as_deref() != Ok("windows") {
        return;
    }
    let out = PathBuf::from(env::var_os("OUT_DIR").expect("cargo sets OUT_DIR"));
    let ico = out.join("archeion.ico");
    fs::write(&ico, icon::ico(&[16, 20, 24, 32, 40, 48, 64, 256])).expect("writing the icon");
    let mut res = winresource::WindowsResource::new();
    res.set_icon(ico.to_str().expect("OUT_DIR is valid Unicode"));
    // What the file's Properties show (the version comes from Cargo.toml).
    res.set("ProductName", "Archeion");
    res.set("FileDescription", "Archeion backup");
    res.set("LegalCopyright", "Copyright (C) 2026 MR. GNU GPL version 3 only.");
    res.compile().expect("compiling the icon resource");
}
