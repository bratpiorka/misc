//===----------------------------------------------------------------------===//
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

    pub unsafe fn alloc_zeros<T: ValidAsZeroBits>(
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
