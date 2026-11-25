fn main() {
  // Linux .deb: the libmpv plugin looks for libmpv-wrapper.so beside the exe, then loads it by name, and the loader
  // looks in the exe's RUNPATH. The .deb puts the wrapper in /usr/lib/qistara (tauri.linux.conf.json). The AppImage
  // and `tauri dev` keep it in <exe>/lib, where the plugin looks first.
  if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("linux") {
    println!("cargo:rustc-link-arg-bins=-Wl,-rpath,$ORIGIN/../lib/qistara");
  }
  tauri_build::build()
}
