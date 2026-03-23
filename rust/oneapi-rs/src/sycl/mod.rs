//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

//! Rust bindings for a small SYCL runtime surface.
//!
//! The low-level FFI lives in [sys], error translation in [result], and the
//! owned Rust abstractions in [safe].

pub mod result;
pub mod safe;
#[allow(warnings)]
pub mod sys;

pub use self::safe::{
    memcpy, memcpy_sync, DeviceCopy, DevicePtr, DevicePtrMut, MemcpyDestination, MemcpySource,
    SyclBuffer, SyclContext, SyclDevice, SyclEvent, SyclKernel, SyclKernelArg, SyclProgram,
    SyclQueue,
};
