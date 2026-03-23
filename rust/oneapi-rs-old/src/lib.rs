#[link(name = "ur_loader")]
extern "C" {}

pub mod unified_runtime;

pub use unified_runtime::result;
pub use unified_runtime::safe;
pub use unified_runtime::sys;

