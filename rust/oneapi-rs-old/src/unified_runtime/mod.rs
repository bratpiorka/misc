//! Wrappers around the oneAPI Unified Runtime API,
//! in two levels: an unsafe low-level API and a thin wrapper around it.

pub mod result;
pub mod safe;
#[allow(warnings)]
pub mod sys;

pub use self::safe::UrContext;

