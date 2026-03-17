//! Safe abstractions over [crate::unified_runtime::result].

pub(crate) mod core;

pub use self::core::{Context, Queue, UrDeviceSlice};
pub use crate::unified_runtime::result::UnifiedRuntimeError;
