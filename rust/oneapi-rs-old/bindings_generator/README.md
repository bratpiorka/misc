# Unified Runtime Bindings Generator

This crate generates Rust FFI bindings for the Unified Runtime C API declared in `ur_api.h`.

By default it reads the checked-in Unified Runtime header from:

`../../unified-runtime/include/unified-runtime/ur_api.h`

and writes the generated Rust file to:

`out/ur_api.rs`

## Usage

```bash
cargo run --release
```

Override the defaults when needed:

```bash
cargo run --release -- \
  --include-dir ../../unified-runtime/include \
  --header include/wrapper.h \
  --output out/ur_api.rs
```
