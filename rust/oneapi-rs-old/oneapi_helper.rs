#![allow(dead_code)]

use std::{env, path::{Path, PathBuf}};

pub const ONEAPI_ROOT: &str = "/home/rrudnick/oneapi_2026.0.0.391";

pub fn compiler_lib_dir() -> PathBuf {
    PathBuf::from(ONEAPI_ROOT).join("compiler/latest/lib")
}

pub fn compiler_opt_lib_dir() -> PathBuf {
    PathBuf::from(ONEAPI_ROOT).join("compiler/latest/opt/compiler/lib")
}

pub fn compiler_bin_dir() -> PathBuf {
    PathBuf::from(ONEAPI_ROOT).join("compiler/latest/bin")
}

pub fn compiler_bin_llvm_dir() -> PathBuf {
    PathBuf::from(ONEAPI_ROOT).join("compiler/latest/bin-llvm")
}

pub fn umf_lib_dir() -> PathBuf {
    PathBuf::from(ONEAPI_ROOT).join("umf/latest/lib")
}

pub fn tcm_lib_dir() -> PathBuf {
    PathBuf::from(ONEAPI_ROOT).join("tcm/latest/lib")
}

pub fn level_zero_adapter_path() -> PathBuf {
    compiler_lib_dir().join("libur_adapter_level_zero.so")
}

pub fn opencl_icd_path() -> PathBuf {
    compiler_lib_dir().join("libintelocl.so")
}

pub fn runtime_dirs() -> [PathBuf; 4] {
    [
        compiler_lib_dir(),
        compiler_opt_lib_dir(),
        umf_lib_dir(),
        tcm_lib_dir(),
    ]
}

pub fn configure_runtime_environment() {
    env::set_var("ONEAPI_ROOT", ONEAPI_ROOT);

    if env::var_os("UR_ADAPTERS_SEARCH_PATH")
        .as_deref()
        .map_or(true, |value| value.is_empty())
    {
        env::set_var("UR_ADAPTERS_SEARCH_PATH", compiler_lib_dir());
    }

    let opencl_icd = opencl_icd_path();
    if opencl_icd.is_file()
        && env::var_os("OCL_ICD_FILENAMES")
            .as_deref()
            .map_or(true, |value| value.is_empty())
    {
        env::set_var("OCL_ICD_FILENAMES", opencl_icd);
    }

    prepend_env_path("PATH", &compiler_bin_dir());
    prepend_env_path("PATH", &compiler_bin_llvm_dir());

    for directory in runtime_dirs() {
        prepend_env_path("LD_LIBRARY_PATH", &directory);
        prepend_env_path("LIBRARY_PATH", &directory);
    }
}

pub fn configure_level_zero_runtime_environment() {
    configure_runtime_environment();

    if env::var_os("UR_ADAPTERS_FORCE_LOAD")
        .as_deref()
        .map_or(true, |value| value.is_empty())
    {
        env::set_var("UR_ADAPTERS_FORCE_LOAD", level_zero_adapter_path());
    }

    if env::var_os("ONEAPI_DEVICE_SELECTOR")
        .as_deref()
        .map_or(true, |value| value.is_empty())
    {
        env::set_var("ONEAPI_DEVICE_SELECTOR", "level_zero:gpu");
    }
}

pub fn resolve_ocloc_path() -> Result<PathBuf, String> {
    if let Some(path) = env::var_os("OCLOC_PATH") {
        let path = PathBuf::from(path);
        if path.is_file() {
            return Ok(path);
        }

        return Err(format!(
            "OCLOC_PATH is set but does not point to a file: {}",
            path.display()
        ));
    }

    let oneapi_ocloc = compiler_bin_llvm_dir().join("ocloc");
    if oneapi_ocloc.is_file() {
        return Ok(oneapi_ocloc);
    }

    let path_env = env::var_os("PATH").ok_or_else(|| {
        "PATH is not set; install intel-ocloc or set OCLOC_PATH to the ocloc binary"
            .to_string()
    })?;

    for directory in env::split_paths(&path_env) {
        let candidate = directory.join("ocloc");
        if candidate.is_file() {
            return Ok(candidate);
        }
    }

    Err("ocloc was not found in PATH; install intel-ocloc or set OCLOC_PATH".to_string())
}

pub fn prepend_env_path(key: &str, path: &Path) {
    if !path.is_dir() {
        return;
    }

    let has_path = env::var_os(key)
        .as_deref()
        .map_or(false, |value| env::split_paths(value).any(|entry| entry == path));

    if !has_path {
        env::set_var(key, join_env_path(key, path));
    }
}

pub fn join_env_path(key: &str, extra_dir: &Path) -> String {
    match env::var_os(key) {
        Some(existing) if !existing.is_empty() => {
            format!(
                "{}:{}",
                extra_dir.display(),
                PathBuf::from(existing).display()
            )
        }
        _ => extra_dir.display().to_string(),
    }
}

pub fn emit_runtime_rpath_args() {
    for directory in runtime_dirs() {
        if directory.is_dir() {
            println!("cargo:rustc-link-arg=-Wl,-rpath,{}", directory.display());
        }
    }
}

pub fn emit_ur_loader_link_settings(manifest_dir: &Path) {
    println!("cargo:rerun-if-env-changed=UR_LOADER_LIB_DIR");
    println!("cargo:rustc-link-lib=dylib=ur_loader");

    if let Ok(lib_dir) = env::var("UR_LOADER_LIB_DIR") {
        println!("cargo:rustc-link-search=native={lib_dir}");
        return;
    }

    let oneapi_lib_dir = compiler_lib_dir();
    if oneapi_lib_dir.is_dir() {
        println!(
            "cargo:rustc-link-search=native={}",
            oneapi_lib_dir.display()
        );
        return;
    }

    let fallback_lib_dir = manifest_dir.join("../unified-runtime/build-cuda/install/lib");
    if fallback_lib_dir.is_dir() {
        println!(
            "cargo:rustc-link-search=native={}",
            fallback_lib_dir.display()
        );
    }
}