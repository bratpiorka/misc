//! Safe abstractions over [crate::unified_runtime::result].

pub(crate) mod core;

pub use self::core::{
	DevicePtr, DevicePtrMut, DeviceRepr, UrContext, UrDeviceSlice, UrEvent,
	UrEventStatus, UrKernel, UrProgram, UrQueue, ValidAsZeroBits,
};
pub use crate::unified_runtime::result::UrError;
