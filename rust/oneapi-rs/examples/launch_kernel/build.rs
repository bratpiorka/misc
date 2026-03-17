#[path = "../../oneapi_helper.rs"]
mod oneapi_helper;

fn main() {
    oneapi_helper::emit_runtime_rpath_args();
}