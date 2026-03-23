//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

use std::{
    env,
    path::{Path, PathBuf},
};

const DEFAULT_ONEAPI_ROOT: &str = "/home/rrudnick/oneapi_2026.0.0.391";
const SYCL_SHIM_CPP: &str = "src/sycl/sys/shim.cpp";
const SYCL_WRAPPER_H: &str = "src/sycl/sys/wrapper.h";

fn main() {
    println!("cargo:rerun-if-env-changed=ONEAPI_ROOT");
    println!("cargo:rerun-if-changed={SYCL_SHIM_CPP}");
    println!("cargo:rerun-if-changed={SYCL_WRAPPER_H}");

    let oneapi_root = resolve_oneapi_root();
    let include_dir = oneapi_root.join("compiler/2026.0/include");
    let lib_dir = oneapi_root.join("compiler/2026.0/lib");
    let compiler = resolve_sycl_compiler(&oneapi_root);
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let shim_cpp = manifest_dir.join(SYCL_SHIM_CPP);
    let wrapper_h = manifest_dir.join(SYCL_WRAPPER_H);

    if !shim_cpp.is_file() || !wrapper_h.is_file() {
        panic!(
            "generated SYCL sources are missing; run cargo run --manifest-path bindings_generator/sycl/Cargo.toml"
        );
    }

    if !include_dir.join("sycl/sycl.hpp").is_file() {
        panic!(
            "SYCL headers not found at {}; set ONEAPI_ROOT to a sourced oneAPI installation",
            include_dir.display()
        );
    }

    if !lib_dir.is_dir() {
        panic!(
            "SYCL library directory not found at {}; set ONEAPI_ROOT to a sourced oneAPI installation",
            lib_dir.display()
        );
    }

    let mut build = cc::Build::new();
    build
        .cpp(true)
        .file(&shim_cpp)
        .include(&include_dir)
        .std("c++17")
        .flag("-fsycl")
        .compiler(compiler);

    build.compile("oneapi_sycl_shim");

    println!("cargo:rustc-link-search=native={}", lib_dir.display());
    println!("cargo:rustc-link-lib=dylib=sycl");
    println!("cargo:rustc-link-arg=-Wl,-rpath,{}", lib_dir.display());
}

fn resolve_oneapi_root() -> PathBuf {
    let from_env = env::var_os("ONEAPI_ROOT").map(PathBuf::from);
    let fallback = PathBuf::from(DEFAULT_ONEAPI_ROOT);

    match from_env {
        Some(path) if path.is_dir() => path,
        Some(path) => panic!("ONEAPI_ROOT does not point to a directory: {}", path.display()),
        None if fallback.is_dir() => fallback,
        None => panic!(
            "ONEAPI_ROOT is not set and default installation was not found at {}",
            fallback.display()
        ),
    }
}

fn resolve_sycl_compiler(oneapi_root: &Path) -> PathBuf {
    let candidates = [
        oneapi_root.join("compiler/2026.0/bin/icpx"),
        oneapi_root.join("compiler/2026.0/bin/dpcpp"),
    ];

    for candidate in candidates {
        if candidate.is_file() {
            return candidate;
        }
    }

    panic!(
        "no SYCL-capable compiler found under {}; expected icpx or dpcpp",
        oneapi_root.join("compiler/2026.0/bin").display()
    );
}