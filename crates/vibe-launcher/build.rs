//! Embed each launcher's family icon into ITS OWN executable (PROP-043 #icon):
//! `vibetree.exe` carries the emerald tile, `vibeterm.exe` the coral prompt. One
//! crate builds several GUI launchers, so a crate-wide embed (winres) cannot
//! distinguish them — instead we generate a tiny `.rc` per icon and link it only
//! into the matching binary via `cargo:rustc-link-arg-bin` (embed-resource's
//! `compile_for`). Adding a launcher = one row below. A missing resource
//! compiler degrades to a generic icon (a warning), never a build break.
fn main() {
    #[cfg(windows)]
    {
        use std::io::Write;

        // (binary name, icon file under ../../assets/icons/). The `.ico`'s 256
        // layer is the Start-menu tile.
        const LAUNCHERS: &[(&str, &str)] = &[
            ("vibetree", "vibetree.ico"),
            ("vibeterm", "vibeterm.ico"),
            ("vibeframe", "vibeterm.ico"),
        ];

        let manifest = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR");
        let out = std::env::var("OUT_DIR").expect("OUT_DIR");

        for &(bin, ico) in LAUNCHERS {
            // rc.exe accepts forward slashes; an absolute path avoids any CWD
            // ambiguity when embed-resource invokes the compiler.
            let ico_abs = format!("{manifest}/../../assets/icons/{ico}").replace('\\', "/");
            let rc_path = std::path::Path::new(&out).join(format!("{bin}.rc"));
            // Resource id 1, type ICON — the icon Windows shows for the exe.
            let mut f = std::fs::File::create(&rc_path).expect("create launcher .rc");
            writeln!(f, "1 ICON \"{ico_abs}\"").expect("write launcher .rc");
            drop(f);

            // Link this .rc's icon ONLY into `bin` (not the whole crate).
            if let Err(e) = embed_resource::compile_for(&rc_path, [bin], embed_resource::NONE)
                .manifest_optional()
            {
                println!("cargo:warning=vibe-launcher: icon embed for {bin} skipped ({e})");
            }
            println!("cargo:rerun-if-changed=../../assets/icons/{ico}");
        }
    }
}
