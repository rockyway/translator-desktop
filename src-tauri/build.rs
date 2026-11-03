fn main() {
    let mut attributes = tauri_build::Attributes::new();
    // Test binaries import comctl32!TaskDialogIndirect (rfd, tray) but tauri-build's
    // manifest resource reaches only the app exe, so `cargo test` loaded comctl32 v5
    // and died with STATUS_ENTRYPOINT_NOT_FOUND. Embed tauri's default manifest (just
    // the Common-Controls v6 dependency) for Windows MSVC targets; link.exe's generated
    // manifest also carries the default asInvoker trustInfo (equivalent to no trustInfo),
    // verified in the built exe.
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows")
        && std::env::var("CARGO_CFG_TARGET_ENV").as_deref() == Ok("msvc")
    {
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg=/MANIFESTDEPENDENCY:type='win32' \
             name='Microsoft.Windows.Common-Controls' version='6.0.0.0' \
             processorArchitecture='*' publicKeyToken='6595b64144ccf1df' language='*'"
        );
    }
    tauri_build::try_build(attributes).expect("failed to run tauri-build");
}
