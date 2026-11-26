// torch-sys reports the libtorch lib directory it linked against, which is the copy that
// `download-libtorch` put under target/ (or $LIBTORCH/lib if set). Embedding it as an rpath
// lets binaries, tests and debuggers find the dylibs without DYLD_LIBRARY_PATH/LD_LIBRARY_PATH.
fn main() {
    println!("cargo:rerun-if-env-changed=DEP_TCH_LIBTORCH_LIB");
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    if target_os == "windows" {
        return; // no rpath on Windows; the DLLs must be next to the exe or on PATH
    }
    if let Ok(dir) = std::env::var("DEP_TCH_LIBTORCH_LIB") {
        println!("cargo:rustc-link-arg=-Wl,-rpath,{dir}");
    }
}
