fn main() {
    leo3_build_config::use_leo3_cfgs();

    if cfg!(target_os = "windows") {
        return;
    }

    let origin = if cfg!(target_os = "macos") {
        "@loader_path"
    } else {
        "$ORIGIN"
    };
    println!("cargo:rustc-link-arg=-Wl,-rpath,{origin}");
}
