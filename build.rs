fn main() {
    // Compile and embed Win32 resources from res/app.rc
    // Includes: menu, accelerators, version info, and manifest.
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=res/app.rc");
        println!("cargo:rerun-if-changed=res/cards.png");
        println!("cargo:rerun-if-changed=res/app.manifest");
        println!("cargo:rerun-if-changed=res/app.ico");
        embed_resource::compile("res/app.rc", embed_resource::NONE);
    }
}
