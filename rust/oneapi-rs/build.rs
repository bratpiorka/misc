use std::{env, path::PathBuf};

fn main() {
    println!("cargo:rerun-if-env-changed=UR_LOADER_LIB_DIR");
    println!("cargo:rustc-link-lib=dylib=ur_loader");

    if let Ok(lib_dir) = env::var("UR_LOADER_LIB_DIR") {
        println!("cargo:rustc-link-search=native={lib_dir}");
        return;
    }

    let default_lib_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../unified-runtime/build-cuda/install/lib");
    if default_lib_dir.is_dir() {
        println!(
            "cargo:rustc-link-search=native={}",
            default_lib_dir.display()
        );
    }
}
