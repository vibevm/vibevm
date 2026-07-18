//! Embed the shared family icon into the Windows executable (PROP-043 #icon):
//! the multi-resolution `.ico` whose 256 layer is the Start-menu tile. A missing
//! resource compiler degrades to a generic icon (a warning), never a build break.
fn main() {
    #[cfg(windows)]
    {
        let mut res = winres::WindowsResource::new();
        res.set_icon("../../assets/icons/vibetree.ico");
        if let Err(e) = res.compile() {
            println!("cargo:warning=vibe-launcher: icon embed skipped ({e})");
        }
        println!("cargo:rerun-if-changed=../../assets/icons/vibetree.ico");
    }
}
