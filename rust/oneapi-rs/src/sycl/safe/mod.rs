//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

//! Safe abstractions over [crate::sycl::result].

pub(crate) mod core;

pub use self::core::{
    memcpy, memcpy_sync, DeviceCopy, DevicePtr, DevicePtrMut, MemcpyDestination, MemcpySource,
    SyclBuffer, SyclContext, SyclDevice, SyclEvent, SyclKernel, SyclKernelArg, SyclProgram,
    SyclQueue,
};
pub use crate::sycl::result::SyclError;
