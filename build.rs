fn main() {
    // Compile and embed Win32 resources from res/app.rc
    // Includes: menu, accelerators, version info, and manifest.
    #[cfg(windows)]
    {
        embed_resource::compile("res/app.rc", embed_resource::NONE);
    }
}
