// TODO update to new structure

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use bindgen::{Builder, EnumVariation, Formatter};
use clap::Parser;

#[derive(Debug, Parser)]
#[command(author, version, about = "Generate Rust bindings for Unified Runtime")]
struct Args {
    /// Header or wrapper passed to bindgen.
    #[arg(long, value_name = "PATH")]
    header: Option<PathBuf>,

    /// Include root containing unified-runtime/ur_api.h.
    #[arg(long, value_name = "PATH")]
    include_dir: Option<PathBuf>,

    /// Output Rust file for the generated bindings.
    #[arg(long, value_name = "PATH")]
    output: Option<PathBuf>,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let paths = Paths::from_args(args)?;

    validate_inputs(&paths)?;

    if let Some(parent) = paths.output.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    let bindings = generate_bindings(&paths)?;
    bindings
        .write_to_file(&paths.output)
        .with_context(|| format!("failed to write {}", paths.output.display()))?;

    println!("Generated {}", paths.output.display());
    Ok(())
}

#[derive(Debug)]
struct Paths {
    header: PathBuf,
    include_dir: PathBuf,
    output: PathBuf,
}

impl Paths {
    fn from_args(args: Args) -> Result<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let default_header = manifest_dir.join("include/wrapper.h");
        let default_include_dir = manifest_dir.join("../../unified-runtime/include");
        let default_output = manifest_dir.join("out/ur_api.rs");

        Ok(Self {
            header: normalize_path(args.header.unwrap_or(default_header)),
            include_dir: normalize_path(args.include_dir.unwrap_or(default_include_dir)),
            output: normalize_path(args.output.unwrap_or(default_output)),
        })
    }
}

fn normalize_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
    }
}

fn validate_inputs(paths: &Paths) -> Result<()> {
    ensure_file_exists(&paths.header, "header")?;
    ensure_dir_exists(&paths.include_dir, "include directory")?;

    let ur_header = paths.include_dir.join("unified-runtime/ur_api.h");
    ensure_file_exists(&ur_header, "Unified Runtime header")?;
    Ok(())
}

fn ensure_file_exists(path: &Path, label: &str) -> Result<()> {
    if !path.is_file() {
        bail!("{} not found: {}", label, path.display());
    }
    Ok(())
}

fn ensure_dir_exists(path: &Path, label: &str) -> Result<()> {
    if !path.is_dir() {
        bail!("{} not found: {}", label, path.display());
    }
    Ok(())
}

fn generate_bindings(paths: &Paths) -> Result<bindgen::Bindings> {
    Builder::default()
        .header(paths.header.display().to_string())
        .clang_arg(format!("-I{}", paths.include_dir.display()))
        .allowlist_type("^ur_.*")
        .allowlist_function("^ur.*")
        .allowlist_var("^UR_.*")
        .default_enum_style(EnumVariation::Rust {
            non_exhaustive: false,
        })
        .formatter(Formatter::Prettyplease)
        .generate_comments(false)
        .layout_tests(false)
        .size_t_is_usize(true)
        .use_core()
        .raw_line("#![allow(non_camel_case_types)]")
        .raw_line("#![allow(non_snake_case)]")
        .raw_line("#![allow(non_upper_case_globals)]")
        .raw_line("#![allow(dead_code)]")
        .generate()
        .context("failed to generate Unified Runtime bindings")
}
