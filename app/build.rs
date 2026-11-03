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
    // Runs as the user (never elevated), tells Windows it knows Windows 10
    // and 11 (no compatibility shims, true version numbers), takes paths
    // longer than 260 characters where Windows allows them, and draws at
    // the scale of each monitor.
    res.set_manifest(MANIFEST);
    res.compile().expect("compiling the icon resource");
}

const MANIFEST: &str = r#"<?xml version="1.0" encoding="UTF-8" standalone="yes"?>
<assembly xmlns="urn:schemas-microsoft-com:asm.v1" manifestVersion="1.0">
  <assemblyIdentity type="win32" name="Archeion" version="1.0.0.0"/>
  <trustInfo xmlns="urn:schemas-microsoft-com:asm.v3">
    <security>
      <requestedPrivileges>
        <requestedExecutionLevel level="asInvoker" uiAccess="false"/>
      </requestedPrivileges>
    </security>
  </trustInfo>
  <compatibility xmlns="urn:schemas-microsoft-com:compatibility.v1">
    <application>
      <supportedOS Id="{8e0f7a12-bfb3-4fe8-b9a5-48fd50a15a9a}"/>
    </application>
  </compatibility>
  <application xmlns="urn:schemas-microsoft-com:asm.v3">
    <windowsSettings>
      <longPathAware xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">true</longPathAware>
      <dpiAwareness xmlns="http://schemas.microsoft.com/SMI/2016/WindowsSettings">PerMonitorV2</dpiAwareness>
    </windowsSettings>
  </application>
  <dependency>
    <dependentAssembly>
      <assemblyIdentity type="win32" name="Microsoft.Windows.Common-Controls" version="6.0.0.0"
        processorArchitecture="*" publicKeyToken="6595b64144ccf1df" language="*"/>
    </dependentAssembly>
  </dependency>
</assembly>
"#;
