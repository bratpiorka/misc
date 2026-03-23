mod oneapi_helper;

use std::path::PathBuf;

fn main() {
    oneapi_helper::emit_ur_loader_link_settings(&PathBuf::from(env!("CARGO_MANIFEST_DIR")));
}
