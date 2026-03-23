//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

use std::{
    env, fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result, bail};
use bindgen::{Builder, EnumVariation, Formatter};
use clap::Parser;

const DEFAULT_ONEAPI_ROOT: &str = "/home/rrudnick/oneapi_2026.0.0.391";
const SYCL_HEADER_RELATIVE: &str = "compiler/2026.0/include/sycl/sycl.hpp";
const LLVM_HEADER: &str = "//===----------------------------------------------------------------------===//\n//\n// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.\n// See https://llvm.org/LICENSE.txt for license information.\n// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception\n//\n//===----------------------------------------------------------------------===//\n";

const MODULE_MOD_RS: &str = r#"//===----------------------------------------------------------------------===//
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
    SyclQueue, ValidAsZeroBits,
};
"#;

const RESULT_RS: &str = r#"//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

//! Error translation for the low-level SYCL shim.

use std::ffi::CStr;

use super::sys;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SyclError {
    pub code: sys::sycl_rs_result_t,
    pub message: String,
}

impl SyclError {
    fn new(code: sys::sycl_rs_result_t) -> Self {
        Self {
            code,
            message: last_error_message(),
        }
    }
}

impl sys::sycl_rs_result_t {
    #[inline]
    pub fn result(self) -> Result<(), SyclError> {
        match self {
            sys::sycl_rs_result_t::SYCL_RS_RESULT_SUCCESS => Ok(()),
            _ => Err(SyclError::new(self)),
        }
    }
}

impl std::fmt::Display for SyclError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.message.is_empty() {
            write!(f, "SYCL error code {}", self.code as u32)
        } else {
            write!(f, "{} (code {})", self.message, self.code as u32)
        }
    }
}

impl std::error::Error for SyclError {}

fn last_error_message() -> String {
    unsafe {
        let ptr = sys::sycl_rs_last_error_message();
        if ptr.is_null() {
            String::new()
        } else {
            CStr::from_ptr(ptr).to_string_lossy().into_owned()
        }
    }
}
"#;

const SAFE_MOD_RS: &str = r#"//===----------------------------------------------------------------------===//
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
    SyclQueue, ValidAsZeroBits,
};
pub use crate::sycl::result::SyclError;
"#;

const SAFE_CORE_RS: &str = r#"//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

use crate::sycl::{result, sys};

use std::{
    ffi::{c_char, c_void, CStr, CString},
    marker::PhantomData,
    ptr,
    sync::Arc,
};

pub unsafe trait DeviceCopy: Copy {}

pub unsafe trait ValidAsZeroBits: DeviceCopy {}

pub unsafe trait DevicePtr<T> {
    fn device_ptr(&self) -> *const T;
}

pub unsafe trait DevicePtrMut<T>: DevicePtr<T> {
    fn device_ptr_mut(&mut self) -> *mut T;
}

unsafe impl DeviceCopy for bool {}
unsafe impl DeviceCopy for i8 {}
unsafe impl DeviceCopy for i16 {}
unsafe impl DeviceCopy for i32 {}
unsafe impl DeviceCopy for i64 {}
unsafe impl DeviceCopy for i128 {}
unsafe impl DeviceCopy for isize {}
unsafe impl DeviceCopy for u8 {}
unsafe impl DeviceCopy for u16 {}
unsafe impl DeviceCopy for u32 {}
unsafe impl DeviceCopy for u64 {}
unsafe impl DeviceCopy for u128 {}
unsafe impl DeviceCopy for usize {}
unsafe impl DeviceCopy for f32 {}
unsafe impl DeviceCopy for f64 {}
unsafe impl<T: DeviceCopy, const N: usize> DeviceCopy for [T; N] {}

unsafe impl ValidAsZeroBits for bool {}
unsafe impl ValidAsZeroBits for i8 {}
unsafe impl ValidAsZeroBits for i16 {}
unsafe impl ValidAsZeroBits for i32 {}
unsafe impl ValidAsZeroBits for i64 {}
unsafe impl ValidAsZeroBits for i128 {}
unsafe impl ValidAsZeroBits for isize {}
unsafe impl ValidAsZeroBits for u8 {}
unsafe impl ValidAsZeroBits for u16 {}
unsafe impl ValidAsZeroBits for u32 {}
unsafe impl ValidAsZeroBits for u64 {}
unsafe impl ValidAsZeroBits for u128 {}
unsafe impl ValidAsZeroBits for usize {}
unsafe impl ValidAsZeroBits for f32 {}
unsafe impl ValidAsZeroBits for f64 {}
unsafe impl<T: ValidAsZeroBits, const N: usize> ValidAsZeroBits for [T; N] {}

pub trait MemcpySource<T> {
    fn len(&self) -> usize;
    fn as_memcpy_src(&self) -> *const c_void;
}

pub trait MemcpyDestination<T> {
    fn len(&self) -> usize;
    fn as_memcpy_dst(&self) -> *mut c_void;
}

pub fn memcpy<T, S, D>(queue: &SyclQueue, src: S, dst: D) -> Result<SyclEvent, result::SyclError>
where
    S: MemcpySource<T>,
    D: MemcpyDestination<T>,
{
    if src.len() != dst.len() {
        return Err(length_mismatch());
    }

    queue.memcpy_bytes_async(
        dst.as_memcpy_dst(),
        src.as_memcpy_src(),
        bytes_for_len::<T>(src.len())?,
    )
}

pub fn memcpy_sync<T, S, D>(queue: &SyclQueue, src: S, dst: D) -> Result<(), result::SyclError>
where
    S: MemcpySource<T>,
    D: MemcpyDestination<T>,
{
    memcpy(queue, src, dst)?.wait()
}

#[derive(Debug)]
pub struct SyclDevice {
    pub(crate) handle: *mut sys::sycl_rs_device_t,
}

#[derive(Debug)]
pub struct SyclEvent {
    pub(crate) handle: *mut sys::sycl_rs_event_t,
}

unsafe impl Send for SyclEvent {}
unsafe impl Sync for SyclEvent {}

impl Drop for SyclEvent {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::sycl_rs_event_destroy(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

impl SyclEvent {
    pub fn wait(&self) -> Result<(), result::SyclError> {
        unsafe { sys::sycl_rs_event_wait(self.handle).result() }
    }

    pub fn handle(&self) -> *mut sys::sycl_rs_event_t {
        self.handle
    }
}

unsafe impl Send for SyclDevice {}
unsafe impl Sync for SyclDevice {}

impl Drop for SyclDevice {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::sycl_rs_device_destroy(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

impl SyclDevice {
    pub fn count() -> Result<usize, result::SyclError> {
        let mut count = 0;
        unsafe { sys::sycl_rs_device_count(&mut count).result()? };
        Ok(count)
    }

    pub fn by_ordinal(ordinal: usize) -> Result<Arc<Self>, result::SyclError> {
        let mut handle = ptr::null_mut();
        unsafe { sys::sycl_rs_device_create_with_index(ordinal, &mut handle).result()? };
        Ok(Arc::new(Self { handle }))
    }

    pub fn default() -> Result<Arc<Self>, result::SyclError> {
        let mut handle = ptr::null_mut();
        unsafe { sys::sycl_rs_device_create_default(&mut handle).result()? };
        Ok(Arc::new(Self { handle }))
    }

    pub fn handle(&self) -> *mut sys::sycl_rs_device_t {
        self.handle
    }
}

#[derive(Debug)]
pub struct SyclContext {
    pub(crate) handle: *mut sys::sycl_rs_context_t,
    pub(crate) device: Arc<SyclDevice>,
}

unsafe impl Send for SyclContext {}
unsafe impl Sync for SyclContext {}

impl Drop for SyclContext {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::sycl_rs_context_destroy(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

impl SyclContext {
    pub fn device_count() -> Result<usize, result::SyclError> {
        SyclDevice::count()
    }

    pub fn from_device_ordinal(ordinal: usize) -> Result<Arc<Self>, result::SyclError> {
        let device = SyclDevice::by_ordinal(ordinal)?;
        Self::new(&device)
    }

    pub fn new(device: &Arc<SyclDevice>) -> Result<Arc<Self>, result::SyclError> {
        let mut handle = ptr::null_mut();
        unsafe { sys::sycl_rs_context_create(device.handle, &mut handle).result()? };
        Ok(Arc::new(Self {
            handle,
            device: Arc::clone(device),
        }))
    }

    pub fn device(&self) -> &Arc<SyclDevice> {
        &self.device
    }

    pub fn handle(&self) -> *mut sys::sycl_rs_context_t {
        self.handle
    }

    pub fn load_program_from_source(
        self: &Arc<Self>,
        source: &str,
    ) -> Result<Arc<SyclProgram>, result::SyclError> {
        let mut handle = ptr::null_mut();
        unsafe {
            sys::sycl_rs_program_build_from_source(
                self.handle,
                self.device.handle,
                source.as_ptr().cast::<c_char>(),
                source.len(),
                ptr::null(),
                &mut handle,
            )
            .result()?
        };

        Ok(Arc::new(SyclProgram {
            handle,
            context: Arc::clone(self),
        }))
    }
}

#[derive(Debug)]
pub struct SyclQueue {
    pub(crate) handle: *mut sys::sycl_rs_queue_t,
    pub(crate) context: Arc<SyclContext>,
}

unsafe impl Send for SyclQueue {}
unsafe impl Sync for SyclQueue {}

impl Drop for SyclQueue {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::sycl_rs_queue_destroy(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

impl SyclQueue {
    pub fn new_for_device_ordinal(ordinal: usize) -> Result<Arc<Self>, result::SyclError> {
        let context = SyclContext::from_device_ordinal(ordinal)?;
        Self::new(&context)
    }

    pub fn new(context: &Arc<SyclContext>) -> Result<Arc<Self>, result::SyclError> {
        let mut handle = ptr::null_mut();
        unsafe {
            sys::sycl_rs_queue_create(context.handle, context.device.handle, &mut handle)
                .result()?
        };
        Ok(Arc::new(Self {
            handle,
            context: Arc::clone(context),
        }))
    }

    pub fn new_default() -> Result<Arc<Self>, result::SyclError> {
        let device = SyclDevice::default()?;
        let context = SyclContext::new(&device)?;
        Self::new(&context)
    }

    pub fn context(&self) -> &Arc<SyclContext> {
        &self.context
    }

    pub fn handle(&self) -> *mut sys::sycl_rs_queue_t {
        self.handle
    }

    pub fn synchronize(&self) -> Result<(), result::SyclError> {
        unsafe { sys::sycl_rs_queue_wait(self.handle).result() }
    }

    pub fn load_program_from_source(
        self: &Arc<Self>,
        source: &str,
    ) -> Result<Arc<SyclProgram>, result::SyclError> {
        self.context.load_program_from_source(source)
    }

    pub unsafe fn alloc_device<T>(
        self: &Arc<Self>,
        len: usize,
    ) -> Result<SyclBuffer<T>, result::SyclError> {
        unsafe { self.alloc(len, sys::sycl_rs_alloc_kind_t::SYCL_RS_ALLOC_KIND_DEVICE) }
    }

    pub fn alloc_zeros<T: ValidAsZeroBits>(
        self: &Arc<Self>,
        len: usize,
    ) -> Result<SyclBuffer<T>, result::SyclError> {
        let buffer = unsafe { self.alloc_device::<T>(len)? };
        let bytes = bytes_for_len::<T>(len)?;
        unsafe { sys::sycl_rs_memset(self.handle, buffer.ptr.cast::<c_void>(), 0, bytes).result()? };
        Ok(buffer)
    }

    pub unsafe fn alloc_shared<T>(
        self: &Arc<Self>,
        len: usize,
    ) -> Result<SyclBuffer<T>, result::SyclError> {
        unsafe { self.alloc(len, sys::sycl_rs_alloc_kind_t::SYCL_RS_ALLOC_KIND_SHARED) }
    }

    pub unsafe fn alloc_host<T>(
        self: &Arc<Self>,
        len: usize,
    ) -> Result<SyclBuffer<T>, result::SyclError> {
        unsafe { self.alloc(len, sys::sycl_rs_alloc_kind_t::SYCL_RS_ALLOC_KIND_HOST) }
    }

    pub fn clone_dtoh<T: DeviceCopy + Default>(
        &self,
        src: &SyclBuffer<T>,
    ) -> Result<Vec<T>, result::SyclError> {
        let mut dst = vec![T::default(); src.len];
        memcpy_sync(self, src, dst.as_mut_slice())?;
        Ok(dst)
    }

    unsafe fn alloc<T>(
        self: &Arc<Self>,
        len: usize,
        kind: sys::sycl_rs_alloc_kind_t,
    ) -> Result<SyclBuffer<T>, result::SyclError> {
        if len == 0 {
            return Ok(SyclBuffer {
                ptr: ptr::null_mut(),
                len,
                queue: Arc::clone(self),
                kind,
                marker: PhantomData,
            });
        }

        let mut ptr = ptr::null_mut();
        let bytes = bytes_for_len::<T>(len)?;
        unsafe {
            sys::sycl_rs_alloc(self.handle, kind, bytes, std::mem::align_of::<T>(), &mut ptr)
                .result()?
        };

        Ok(SyclBuffer {
            ptr: ptr.cast::<T>(),
            len,
            queue: Arc::clone(self),
            kind,
            marker: PhantomData,
        })
    }

    fn memcpy_bytes_async(
        &self,
        dst: *mut c_void,
        src: *const c_void,
        bytes: usize,
    ) -> Result<SyclEvent, result::SyclError> {
        let mut handle = ptr::null_mut();
        unsafe { sys::sycl_rs_memcpy_async(self.handle, dst, src, bytes, &mut handle).result()? };
        Ok(SyclEvent { handle })
    }
}

#[derive(Debug)]
pub struct SyclBuffer<T> {
    pub(crate) ptr: *mut T,
    pub(crate) len: usize,
    pub(crate) queue: Arc<SyclQueue>,
    pub(crate) kind: sys::sycl_rs_alloc_kind_t,
    pub(crate) marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for SyclBuffer<T> {}
unsafe impl<T: Sync> Sync for SyclBuffer<T> {}

impl<T> Drop for SyclBuffer<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let _ = unsafe {
                sys::sycl_rs_free(self.queue.handle, self.ptr.cast::<c_void>()).result()
            };
            self.ptr = ptr::null_mut();
        }
    }
}

impl<T> SyclBuffer<T> {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }

    pub fn queue(&self) -> &Arc<SyclQueue> {
        &self.queue
    }

    pub fn allocation_kind(&self) -> sys::sycl_rs_alloc_kind_t {
        self.kind
    }
}

unsafe impl<T> DevicePtr<T> for SyclBuffer<T> {
    fn device_ptr(&self) -> *const T {
        self.ptr.cast_const()
    }
}

unsafe impl<T> DevicePtrMut<T> for SyclBuffer<T> {
    fn device_ptr_mut(&mut self) -> *mut T {
        self.ptr
    }
}

#[derive(Debug)]
pub struct SyclProgram {
    pub(crate) handle: *mut sys::sycl_rs_program_t,
    pub(crate) context: Arc<SyclContext>,
}

unsafe impl Send for SyclProgram {}
unsafe impl Sync for SyclProgram {}

impl Drop for SyclProgram {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::sycl_rs_program_destroy(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

impl SyclProgram {
    pub fn context(&self) -> &Arc<SyclContext> {
        &self.context
    }

    pub fn build_log(&self) -> String {
        unsafe {
            let ptr = sys::sycl_rs_program_last_log(self.handle);
            if ptr.is_null() {
                String::new()
            } else {
                CStr::from_ptr(ptr).to_string_lossy().into_owned()
            }
        }
    }

    pub fn create_kernel(
        self: &Arc<Self>,
        kernel_name: &str,
    ) -> Result<Arc<SyclKernel>, result::SyclError> {
        let kernel_name = CString::new(kernel_name)
            .map_err(|_| invalid_argument("kernel name contains interior NUL"))?;
        let mut handle = ptr::null_mut();
        unsafe {
            sys::sycl_rs_program_get_kernel(self.handle, kernel_name.as_ptr(), &mut handle)
                .result()?
        };

        Ok(Arc::new(SyclKernel {
            handle,
            program: Arc::clone(self),
        }))
    }
}

#[derive(Debug)]
pub struct SyclKernel {
    pub(crate) handle: *mut sys::sycl_rs_kernel_t,
    pub(crate) program: Arc<SyclProgram>,
}

unsafe impl Send for SyclKernel {}
unsafe impl Sync for SyclKernel {}

impl Drop for SyclKernel {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            unsafe { sys::sycl_rs_kernel_destroy(self.handle) };
            self.handle = ptr::null_mut();
        }
    }
}

impl SyclKernel {
    pub fn program(&self) -> &Arc<SyclProgram> {
        &self.program
    }

    pub unsafe fn launch_1d(
        &self,
        queue: &SyclQueue,
        global_range: usize,
        local_range: usize,
        args: &[SyclKernelArg],
    ) -> Result<(), result::SyclError> {
        let raw_args = args.iter().map(SyclKernelArg::raw).collect::<Vec<_>>();
        unsafe {
            sys::sycl_rs_kernel_launch_1d(
                queue.handle,
                self.handle,
                global_range,
                local_range,
                raw_args.as_ptr(),
                raw_args.len(),
            )
            .result()
        }
    }
}

#[derive(Clone, Debug)]
pub struct SyclKernelArg {
    bytes: Box<[u8]>,
}

impl SyclKernelArg {
    pub fn scalar<T: Copy>(value: &T) -> Self {
        Self::from_copy(value)
    }

    pub fn ptr<T>(value: *const T) -> Self {
        let ptr = value.cast::<c_void>();
        Self::from_copy(&ptr)
    }

    pub fn ptr_mut<T>(value: *mut T) -> Self {
        let ptr = value.cast::<c_void>();
        Self::from_copy(&ptr)
    }

    pub fn device_ptr<T, P>(value: &P) -> Self
    where
        P: DevicePtr<T>,
    {
        Self::ptr(value.device_ptr())
    }

    pub fn device_ptr_mut<T, P>(value: &mut P) -> Self
    where
        P: DevicePtrMut<T>,
    {
        Self::ptr_mut(value.device_ptr_mut())
    }

    pub fn buffer<T>(buffer: &SyclBuffer<T>) -> Self {
        Self::device_ptr(buffer)
    }

    pub fn buffer_mut<T>(buffer: &mut SyclBuffer<T>) -> Self {
        Self::device_ptr_mut(buffer)
    }

    fn from_copy<T: Copy>(value: &T) -> Self {
        let bytes = unsafe {
            std::slice::from_raw_parts(
                (value as *const T).cast::<u8>(),
                std::mem::size_of::<T>(),
            )
        };
        Self {
            bytes: bytes.to_vec().into_boxed_slice(),
        }
    }

    fn raw(&self) -> sys::sycl_rs_raw_kernel_arg_t {
        sys::sycl_rs_raw_kernel_arg_t {
            data: self.bytes.as_ptr().cast::<c_void>(),
            size: self.bytes.len(),
        }
    }
}

impl<T: DeviceCopy> MemcpySource<T> for &[T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn as_memcpy_src(&self) -> *const c_void {
        self.as_ptr().cast::<c_void>()
    }
}

impl<T: DeviceCopy, const N: usize> MemcpySource<T> for &[T; N] {
    fn len(&self) -> usize {
        N
    }

    fn as_memcpy_src(&self) -> *const c_void {
        self.as_ptr().cast::<c_void>()
    }
}

impl<T: DeviceCopy> MemcpyDestination<T> for &mut [T] {
    fn len(&self) -> usize {
        <[T]>::len(self)
    }

    fn as_memcpy_dst(&self) -> *mut c_void {
        self.as_ptr().cast_mut().cast::<c_void>()
    }
}

impl<T: DeviceCopy, const N: usize> MemcpyDestination<T> for &mut [T; N] {
    fn len(&self) -> usize {
        N
    }

    fn as_memcpy_dst(&self) -> *mut c_void {
        self.as_ptr().cast_mut().cast::<c_void>()
    }
}

impl<T> MemcpySource<T> for &SyclBuffer<T> {
    fn len(&self) -> usize {
        self.len
    }

    fn as_memcpy_src(&self) -> *const c_void {
        self.ptr.cast::<c_void>()
    }
}

impl<T> MemcpyDestination<T> for &SyclBuffer<T> {
    fn len(&self) -> usize {
        self.len
    }

    fn as_memcpy_dst(&self) -> *mut c_void {
        self.ptr.cast::<c_void>()
    }
}

impl<T> MemcpyDestination<T> for &mut SyclBuffer<T> {
    fn len(&self) -> usize {
        self.len
    }

    fn as_memcpy_dst(&self) -> *mut c_void {
        self.ptr.cast::<c_void>()
    }
}

fn bytes_for_len<T>(len: usize) -> Result<usize, result::SyclError> {
    len.checked_mul(std::mem::size_of::<T>())
        .ok_or_else(length_mismatch)
}

fn invalid_argument(message: &str) -> result::SyclError {
    result::SyclError {
        code: sys::sycl_rs_result_t::SYCL_RS_RESULT_INVALID_ARGUMENT,
        message: message.to_string(),
    }
}

fn length_mismatch() -> result::SyclError {
    invalid_argument("buffer length mismatch")
}
"#;

const WRAPPER_H: &str = r#"//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

#pragma once

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct sycl_rs_device_t sycl_rs_device_t;
typedef struct sycl_rs_context_t sycl_rs_context_t;
typedef struct sycl_rs_queue_t sycl_rs_queue_t;
typedef struct sycl_rs_event_t sycl_rs_event_t;
typedef struct sycl_rs_program_t sycl_rs_program_t;
typedef struct sycl_rs_kernel_t sycl_rs_kernel_t;

typedef enum sycl_rs_result_t {
    SYCL_RS_RESULT_SUCCESS = 0,
    SYCL_RS_RESULT_INVALID_ARGUMENT = 1,
    SYCL_RS_RESULT_OUT_OF_MEMORY = 2,
    SYCL_RS_RESULT_RUNTIME_ERROR = 3,
} sycl_rs_result_t;

typedef enum sycl_rs_alloc_kind_t {
    SYCL_RS_ALLOC_KIND_DEVICE = 0,
    SYCL_RS_ALLOC_KIND_SHARED = 1,
    SYCL_RS_ALLOC_KIND_HOST = 2,
} sycl_rs_alloc_kind_t;

typedef struct sycl_rs_raw_kernel_arg_t {
    const void *data;
    size_t size;
} sycl_rs_raw_kernel_arg_t;

const char *sycl_rs_last_error_message(void);

sycl_rs_result_t sycl_rs_device_count(size_t *out_count);
sycl_rs_result_t sycl_rs_device_create_with_index(size_t index,
                                                  sycl_rs_device_t **out_device);
sycl_rs_result_t sycl_rs_device_create_default(sycl_rs_device_t **out_device);
void sycl_rs_device_destroy(sycl_rs_device_t *device);

sycl_rs_result_t sycl_rs_context_create(
    const sycl_rs_device_t *device,
    sycl_rs_context_t **out_context
);
void sycl_rs_context_destroy(sycl_rs_context_t *context);

sycl_rs_result_t sycl_rs_queue_create(
    const sycl_rs_context_t *context,
    const sycl_rs_device_t *device,
    sycl_rs_queue_t **out_queue
);
void sycl_rs_queue_destroy(sycl_rs_queue_t *queue);

void sycl_rs_event_destroy(sycl_rs_event_t *event);
sycl_rs_result_t sycl_rs_event_wait(const sycl_rs_event_t *event);

sycl_rs_result_t sycl_rs_alloc(
    sycl_rs_queue_t *queue,
    sycl_rs_alloc_kind_t kind,
    size_t bytes,
    size_t alignment,
    void **out_ptr
);
sycl_rs_result_t sycl_rs_free(sycl_rs_queue_t *queue, void *ptr);
sycl_rs_result_t sycl_rs_memset(sycl_rs_queue_t *queue, void *dst, int value,
                                size_t bytes);
sycl_rs_result_t sycl_rs_memcpy(
    sycl_rs_queue_t *queue,
    void *dst,
    const void *src,
    size_t bytes
);
sycl_rs_result_t sycl_rs_memcpy_async(
    sycl_rs_queue_t *queue,
    void *dst,
    const void *src,
    size_t bytes,
    sycl_rs_event_t **out_event
);
sycl_rs_result_t sycl_rs_queue_wait(sycl_rs_queue_t *queue);

sycl_rs_result_t sycl_rs_program_build_from_source(
    const sycl_rs_context_t *context,
    const sycl_rs_device_t *device,
    const char *source,
    size_t source_len,
    const char *build_options,
    sycl_rs_program_t **out_program
);
const char *sycl_rs_program_last_log(const sycl_rs_program_t *program);
void sycl_rs_program_destroy(sycl_rs_program_t *program);

sycl_rs_result_t sycl_rs_program_get_kernel(
    const sycl_rs_program_t *program,
    const char *kernel_name,
    sycl_rs_kernel_t **out_kernel
);
void sycl_rs_kernel_destroy(sycl_rs_kernel_t *kernel);

sycl_rs_result_t sycl_rs_kernel_launch_1d(
    sycl_rs_queue_t *queue,
    const sycl_rs_kernel_t *kernel,
    size_t global_range,
    size_t local_range,
    const sycl_rs_raw_kernel_arg_t *args,
    size_t num_args
);

#ifdef __cplusplus
}
#endif
"#;

const SHIM_CPP: &str = r#"//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

#include "wrapper.h"

#include <sycl/sycl.hpp>

#include <exception>
#include <new>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace syclexp = sycl::ext::oneapi::experimental;

struct sycl_rs_device_t {
    explicit sycl_rs_device_t(sycl::device device) : value(std::move(device)) {}

    sycl::device value;
};

struct sycl_rs_context_t {
    explicit sycl_rs_context_t(sycl::context context) : value(std::move(context)) {}

    sycl::context value;
};

struct sycl_rs_queue_t {
    explicit sycl_rs_queue_t(sycl::queue queue) : value(std::move(queue)) {}

    sycl::queue value;
};

struct sycl_rs_event_t {
    explicit sycl_rs_event_t(sycl::event event) : value(std::move(event)) {}

    sycl::event value;
};

struct sycl_rs_program_t {
    sycl_rs_program_t(sycl::kernel_bundle<sycl::bundle_state::executable> bundle, std::string log)
        : value(std::move(bundle)), build_log(std::move(log)) {}

    sycl::kernel_bundle<sycl::bundle_state::executable> value;
    std::string build_log;
};

struct sycl_rs_kernel_t {
    explicit sycl_rs_kernel_t(sycl::kernel kernel) : value(std::move(kernel)) {}

    sycl::kernel value;
};

namespace {

thread_local std::string g_last_error;

void set_error(const char *message) {
    g_last_error = message != nullptr ? message : "unknown SYCL error";
}

void clear_error() {
    g_last_error.clear();
}

std::vector<sycl::device> enumerate_devices() {
    return sycl::device::get_devices(sycl::info::device_type::all);
}

template <typename Func>
sycl_rs_result_t with_exceptions(Func &&func) {
    try {
        func();
        clear_error();
        return SYCL_RS_RESULT_SUCCESS;
    } catch (const sycl::exception &error) {
        set_error(error.what());
        return SYCL_RS_RESULT_RUNTIME_ERROR;
    } catch (const std::bad_alloc &error) {
        set_error(error.what());
        return SYCL_RS_RESULT_OUT_OF_MEMORY;
    } catch (const std::exception &error) {
        set_error(error.what());
        return SYCL_RS_RESULT_RUNTIME_ERROR;
    } catch (...) {
        set_error("unknown non-standard exception");
        return SYCL_RS_RESULT_RUNTIME_ERROR;
    }
}

sycl_rs_result_t validate_output_pointer(const void *out_ptr) {
    if (out_ptr == nullptr) {
        set_error("output pointer must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return SYCL_RS_RESULT_SUCCESS;
}

}  // namespace

extern "C" {

const char *sycl_rs_last_error_message(void) {
    return g_last_error.c_str();
}

sycl_rs_result_t sycl_rs_device_count(size_t *out_count) {
    if (auto status = validate_output_pointer(out_count); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        *out_count = enumerate_devices().size();
    });
}

sycl_rs_result_t sycl_rs_device_create_with_index(size_t index, sycl_rs_device_t **out_device) {
    if (auto status = validate_output_pointer(out_device); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        auto devices = enumerate_devices();
        if (index >= devices.size()) {
            set_error("device index out of range");
            throw std::invalid_argument("device index out of range");
        }

        *out_device = new sycl_rs_device_t(devices[index]);
    });
}

sycl_rs_result_t sycl_rs_device_create_default(sycl_rs_device_t **out_device) {
    if (auto status = validate_output_pointer(out_device); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        *out_device = new sycl_rs_device_t(sycl::device(sycl::default_selector_v));
    });
}

void sycl_rs_device_destroy(sycl_rs_device_t *device) {
    delete device;
}

sycl_rs_result_t sycl_rs_context_create(
    const sycl_rs_device_t *device,
    sycl_rs_context_t **out_context
) {
    if (device == nullptr) {
        set_error("device must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_context); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        *out_context = new sycl_rs_context_t(sycl::context(device->value));
    });
}

void sycl_rs_context_destroy(sycl_rs_context_t *context) {
    delete context;
}

sycl_rs_result_t sycl_rs_queue_create(
    const sycl_rs_context_t *context,
    const sycl_rs_device_t *device,
    sycl_rs_queue_t **out_queue
) {
    if (context == nullptr) {
        set_error("context must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (device == nullptr) {
        set_error("device must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_queue); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        *out_queue = new sycl_rs_queue_t(sycl::queue(context->value, device->value));
    });
}

void sycl_rs_queue_destroy(sycl_rs_queue_t *queue) {
    delete queue;
}

void sycl_rs_event_destroy(sycl_rs_event_t *event) {
    delete event;
}

sycl_rs_result_t sycl_rs_event_wait(const sycl_rs_event_t *event) {
    if (event == nullptr) {
        set_error("event must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] {
        auto queued_event = event->value;
        queued_event.wait();
    });
}

sycl_rs_result_t sycl_rs_alloc(
    sycl_rs_queue_t *queue,
    sycl_rs_alloc_kind_t kind,
    size_t bytes,
    size_t alignment,
    void **out_ptr
) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_ptr); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        void *ptr = nullptr;
        switch (kind) {
            case SYCL_RS_ALLOC_KIND_DEVICE:
                ptr = alignment > 0 ? sycl::aligned_alloc_device(alignment, bytes, queue->value)
                                    : sycl::malloc_device(bytes, queue->value);
                break;
            case SYCL_RS_ALLOC_KIND_SHARED:
                ptr = alignment > 0 ? sycl::aligned_alloc_shared(alignment, bytes, queue->value)
                                    : sycl::malloc_shared(bytes, queue->value);
                break;
            case SYCL_RS_ALLOC_KIND_HOST:
                ptr = alignment > 0 ? sycl::aligned_alloc_host(alignment, bytes, queue->value)
                                    : sycl::malloc_host(bytes, queue->value);
                break;
            default:
                set_error("unknown allocation kind");
                throw std::invalid_argument("unknown allocation kind");
        }

        if (bytes > 0 && ptr == nullptr) {
            set_error("SYCL allocation returned null");
            throw std::bad_alloc();
        }

        *out_ptr = ptr;
    });
}

sycl_rs_result_t sycl_rs_free(sycl_rs_queue_t *queue, void *ptr) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] { sycl::free(ptr, queue->value); });
}

sycl_rs_result_t sycl_rs_memset(sycl_rs_queue_t *queue, void *dst, int value,
                                size_t bytes) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (bytes > 0 && dst == nullptr) {
        set_error("memset destination must not be null when bytes > 0");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] { queue->value.memset(dst, value, bytes).wait(); });
}

sycl_rs_result_t sycl_rs_memcpy(
    sycl_rs_queue_t *queue,
    void *dst,
    const void *src,
    size_t bytes
) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (bytes > 0 && (dst == nullptr || src == nullptr)) {
        set_error("memcpy source and destination must not be null when bytes > 0");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] { queue->value.memcpy(dst, src, bytes).wait(); });
}

sycl_rs_result_t sycl_rs_memcpy_async(
    sycl_rs_queue_t *queue,
    void *dst,
    const void *src,
    size_t bytes,
    sycl_rs_event_t **out_event
) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (bytes > 0 && (dst == nullptr || src == nullptr)) {
        set_error("memcpy source and destination must not be null when bytes > 0");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_event); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        sycl::event event = queue->value.memcpy(dst, src, bytes);
        *out_event = new sycl_rs_event_t(std::move(event));
    });
}

sycl_rs_result_t sycl_rs_queue_wait(sycl_rs_queue_t *queue) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] { queue->value.wait(); });
}

sycl_rs_result_t sycl_rs_program_build_from_source(
    const sycl_rs_context_t *context,
    const sycl_rs_device_t *device,
    const char *source,
    size_t source_len,
    const char *build_options,
    sycl_rs_program_t **out_program
) {
    if (context == nullptr) {
        set_error("context must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (device == nullptr) {
        set_error("device must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (source == nullptr) {
        set_error("source must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_program); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        auto build_device = device->value;
        if (!build_device.ext_oneapi_can_build(syclexp::source_language::sycl)) {
            set_error("device does not support SYCL runtime compilation");
            throw std::runtime_error("device does not support SYCL runtime compilation");
        }

        auto source_bundle = syclexp::create_kernel_bundle_from_source(
            context->value,
            syclexp::source_language::sycl,
            std::string(source, source_len));

        std::string build_log;
        std::vector<sycl::device> devices{build_device};
        auto executable_bundle =
            (build_options != nullptr && build_options[0] != '\0')
                ? syclexp::build(
                      source_bundle,
                      devices,
                      syclexp::properties{
                          syclexp::build_options{std::string(build_options)},
                          syclexp::save_log(&build_log),
                      })
                : syclexp::build(
                      source_bundle,
                      devices,
                      syclexp::properties{syclexp::save_log(&build_log)});

        *out_program = new sycl_rs_program_t(std::move(executable_bundle), std::move(build_log));
    });
}

const char *sycl_rs_program_last_log(const sycl_rs_program_t *program) {
    return program == nullptr ? nullptr : program->build_log.c_str();
}

void sycl_rs_program_destroy(sycl_rs_program_t *program) {
    delete program;
}

sycl_rs_result_t sycl_rs_program_get_kernel(
    const sycl_rs_program_t *program,
    const char *kernel_name,
    sycl_rs_kernel_t **out_kernel
) {
    if (program == nullptr) {
        set_error("program must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (kernel_name == nullptr) {
        set_error("kernel_name must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_kernel); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        auto bundle = program->value;
        *out_kernel = new sycl_rs_kernel_t(bundle.ext_oneapi_get_kernel(std::string(kernel_name)));
    });
}

void sycl_rs_kernel_destroy(sycl_rs_kernel_t *kernel) {
    delete kernel;
}

sycl_rs_result_t sycl_rs_kernel_launch_1d(
    sycl_rs_queue_t *queue,
    const sycl_rs_kernel_t *kernel,
    size_t global_range,
    size_t local_range,
    const sycl_rs_raw_kernel_arg_t *args,
    size_t num_args
) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (kernel == nullptr) {
        set_error("kernel must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (global_range == 0 || local_range == 0) {
        set_error("global_range and local_range must be greater than zero");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (global_range % local_range != 0) {
        set_error("global_range must be divisible by local_range");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (num_args > 0 && args == nullptr) {
        set_error("args must not be null when num_args > 0");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] {
        queue->value
            .submit([&](sycl::handler &cgh) {
                for (size_t index = 0; index < num_args; ++index) {
                    cgh.set_arg(
                        static_cast<int>(index),
                        syclexp::raw_kernel_arg(args[index].data, args[index].size));
                }
                cgh.parallel_for(
                    sycl::nd_range<1>{{global_range}, {local_range}},
                    kernel->value);
            })
            .wait();
    });
}

}  // extern "C"
"#;

#[derive(Debug, Parser)]
#[command(author, version, about = "Generate SYCL C shim and Rust bindings")]
struct Args {
    /// oneAPI installation root. Defaults to ONEAPI_ROOT, then a known local install.
    #[arg(long, value_name = "PATH")]
    oneapi_root: Option<PathBuf>,

    /// oneapi-rs crate root where src/sycl will be generated.
    #[arg(long, value_name = "PATH")]
    crate_root: Option<PathBuf>,
}

#[derive(Debug)]
struct Paths {
    oneapi_root: PathBuf,
    sycl_header: PathBuf,
    crate_root: PathBuf,
    module_root: PathBuf,
    safe_root: PathBuf,
    sys_root: PathBuf,
}

fn main() -> Result<()> {
    let args = Args::parse();
    let paths = Paths::from_args(args)?;

    validate_inputs(&paths)?;
    ensure_output_dirs(&paths)?;

    write_text(&paths.module_root.join("mod.rs"), MODULE_MOD_RS)?;
    write_text(&paths.module_root.join("result.rs"), RESULT_RS)?;
    write_text(&paths.safe_root.join("mod.rs"), SAFE_MOD_RS)?;
    write_text(&paths.safe_root.join("core.rs"), SAFE_CORE_RS)?;
    write_text(&paths.sys_root.join("wrapper.h"), WRAPPER_H)?;
    write_text(&paths.sys_root.join("shim.cpp"), SHIM_CPP)?;

    let bindings = generate_bindings(&paths)?;
    let rendered_bindings = format!("{LLVM_HEADER}\n{bindings}");
    write_text(&paths.sys_root.join("mod.rs"), &rendered_bindings)
        .context("failed to write generated Rust sys bindings")?;

    println!("Generated {}", paths.module_root.display());
    Ok(())
}

impl Paths {
    fn from_args(args: Args) -> Result<Self> {
        let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let crate_root = normalize_path(
            args.crate_root
                .unwrap_or_else(|| manifest_dir.join("../..")),
        );
        let oneapi_root = resolve_oneapi_root(args.oneapi_root)?;
        let module_root = crate_root.join("src/sycl");
        let safe_root = module_root.join("safe");
        let sys_root = module_root.join("sys");

        Ok(Self {
            sycl_header: oneapi_root.join(SYCL_HEADER_RELATIVE),
            oneapi_root,
            crate_root,
            module_root,
            safe_root,
            sys_root,
        })
    }
}

fn resolve_oneapi_root(arg: Option<PathBuf>) -> Result<PathBuf> {
    if let Some(path) = arg {
        return Ok(normalize_path(path));
    }

    if let Some(path) = env::var_os("ONEAPI_ROOT") {
        return Ok(PathBuf::from(path));
    }

    let fallback = PathBuf::from(DEFAULT_ONEAPI_ROOT);
    if fallback.is_dir() {
        return Ok(fallback);
    }

    bail!(
        "ONEAPI_ROOT is not set and no fallback install exists at {}",
        DEFAULT_ONEAPI_ROOT
    )
}

fn normalize_path(path: PathBuf) -> PathBuf {
    if path.is_absolute() {
        path
    } else {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(path)
    }
}

fn validate_inputs(paths: &Paths) -> Result<()> {
    ensure_dir_exists(&paths.oneapi_root, "oneAPI root")?;
    ensure_file_exists(&paths.sycl_header, "SYCL umbrella header")?;
    ensure_dir_exists(&paths.crate_root, "crate root")?;
    Ok(())
}

fn ensure_output_dirs(paths: &Paths) -> Result<()> {
    fs::create_dir_all(&paths.safe_root)
        .with_context(|| format!("failed to create {}", paths.safe_root.display()))?;
    fs::create_dir_all(&paths.sys_root)
        .with_context(|| format!("failed to create {}", paths.sys_root.display()))?;
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

fn write_text(path: &Path, contents: &str) -> Result<()> {
    fs::write(path, contents).with_context(|| format!("failed to write {}", path.display()))
}

fn generate_bindings(paths: &Paths) -> Result<bindgen::Bindings> {
    Builder::default()
        .header(paths.sys_root.join("wrapper.h").display().to_string())
        .clang_arg("-x")
        .clang_arg("c")
        .allowlist_type("^sycl_rs_.*")
        .allowlist_function("^sycl_rs_.*")
        .allowlist_var("^SYCL_RS_.*")
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
        .context("failed to generate SYCL sys bindings")
}