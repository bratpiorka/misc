use crate::unified_runtime::{result, sys};

use std::{
    ffi::{CString, c_void},
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

static LOADER_INIT: OnceLock<Result<(), result::UrError>> = OnceLock::new();

struct AdapterCache(Vec<sys::ur_adapter_handle_t>);

unsafe impl Send for AdapterCache {}
unsafe impl Sync for AdapterCache {}

static ADAPTER_CACHE: OnceLock<Result<AdapterCache, result::UrError>> = OnceLock::new();

fn ensure_loader_initialized() -> Result<(), result::UrError> {
    *LOADER_INIT.get_or_init(|| result::loader::init(0))
}

fn get_process_adapters() -> Result<Vec<sys::ur_adapter_handle_t>, result::UrError> {
    let adapters = match ADAPTER_CACHE.get_or_init(|| {
        ensure_loader_initialized()?;
        result::adapter::get().map(AdapterCache)
    }) {
        Ok(adapters) => adapters,
        Err(error) => return Err(*error),
    };

    let mut retained = Vec::with_capacity(adapters.0.len());
    for &adapter in &adapters.0 {
        unsafe { result::adapter::retain(adapter)? };
        retained.push(adapter);
    }

    Ok(retained)
}

/// Types for which the all-zero byte pattern is a valid value.
///
/// This is the bound required by [UrQueue::alloc_zeros].
pub unsafe trait ValidAsZeroBits {}

/// Types with a stable device-compatible representation.
///
/// # Safety
/// Implement this only for types whose memory layout matches what kernels expect,
/// typically plain scalars, arrays of device-representable values, or `#[repr(C)]`
/// structs made of such fields.
pub unsafe trait DeviceRepr {}

/// Read-only access to a typed device pointer.
pub trait DevicePtr<T> {
    fn device_ptr(&self) -> *const T;
}

/// Mutable access to a typed device pointer.
pub trait DevicePtrMut<T>: DevicePtr<T> {
    fn device_ptr_mut(&mut self) -> *mut T;
}

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

unsafe impl DeviceRepr for bool {}
unsafe impl DeviceRepr for i8 {}
unsafe impl DeviceRepr for i16 {}
unsafe impl DeviceRepr for i32 {}
unsafe impl DeviceRepr for i64 {}
unsafe impl DeviceRepr for i128 {}
unsafe impl DeviceRepr for isize {}
unsafe impl DeviceRepr for u8 {}
unsafe impl DeviceRepr for u16 {}
unsafe impl DeviceRepr for u32 {}
unsafe impl DeviceRepr for u64 {}
unsafe impl DeviceRepr for u128 {}
unsafe impl DeviceRepr for usize {}
unsafe impl DeviceRepr for f32 {}
unsafe impl DeviceRepr for f64 {}
unsafe impl<T: DeviceRepr, const N: usize> DeviceRepr for [T; N] {}

/// Represents a Unified Runtime context over one or more devices.
///
/// This is the entry point for owning a `ur_context_handle_t` safely in Rust.
#[derive(Debug)]
pub struct UrContext {
    pub(crate) handle: sys::ur_context_handle_t,
    devices: Vec<sys::ur_device_handle_t>,
    adapters: Vec<sys::ur_adapter_handle_t>,
}

unsafe impl Send for UrContext {}
unsafe impl Sync for UrContext {}

impl Drop for UrContext {
    fn drop(&mut self) {
        let handle = std::mem::replace(&mut self.handle, std::ptr::null_mut());
        if !handle.is_null() {
            let _ = unsafe { sys::urContextRelease(handle).result() };
        }

        for device in self.devices.drain(..) {
            let _ = unsafe { sys::urDeviceRelease(device).result() };
        }

        for adapter in self.adapters.drain(..) {
            let _ = unsafe { result::adapter::release(adapter) };
        }
    }
}

impl PartialEq for UrContext {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
            && self.devices == other.devices
            && self.adapters == other.adapters
    }
}

impl Eq for UrContext {}

impl UrContext {
    /// Creates a context from the enumerated device ordinal across all discovered adapters/platforms.
    pub fn new(ordinal: usize) -> Result<Arc<Self>, result::UrError> {
        ensure_loader_initialized()?;

        let adapters = get_process_adapters()?;
        let devices = enumerate_devices_for_adapters(&adapters)?;
        let Some(&device) = devices.get(ordinal) else {
            release_adapters(&adapters);
            for device in devices {
                let _ = unsafe { sys::urDeviceRelease(device).result() };
            }
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_DEVICE,
            ));
        };

        let context = unsafe { Self::from_devices_with_adapters(&[device], adapters) };
        for device in devices {
            let _ = unsafe { sys::urDeviceRelease(device).result() };
        }
        context
    }

    /// Creates a context from a single device handle.
    ///
    /// # Safety
    /// The device handle must be valid for the lifetime of the created context.
    pub unsafe fn from_device(
        device: sys::ur_device_handle_t,
    ) -> Result<Arc<Self>, result::UrError> {
        unsafe { Self::from_devices_with_adapters(&[device], Vec::new()) }
    }

    /// Creates a context from a slice of device handles.
    ///
    /// # Safety
    /// All device handles must be valid and belong to compatible Unified Runtime backends.
    pub unsafe fn from_devices(
        devices: &[sys::ur_device_handle_t],
    ) -> Result<Arc<Self>, result::UrError> {
        unsafe { Self::from_devices_with_adapters(devices, Vec::new()) }
    }

    unsafe fn from_devices_with_adapters(
        devices: &[sys::ur_device_handle_t],
        adapters: Vec<sys::ur_adapter_handle_t>,
    ) -> Result<Arc<Self>, result::UrError> {
        if devices.is_empty() {
            release_adapters(&adapters);
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ));
        }

        for &device in devices {
            unsafe {
                sys::urDeviceRetain(device).result()?;
            }
        }

        let mut handle = std::ptr::null_mut();
        let create_result = unsafe {
            sys::urContextCreate(
                devices.len() as u32,
                devices.as_ptr(),
                std::ptr::null(),
                &mut handle,
            )
            .result()
        };

        if let Err(error) = create_result {
            for &device in devices {
                let _ = unsafe { sys::urDeviceRelease(device).result() };
            }
            release_adapters(&adapters);
            return Err(error);
        }

        Ok(Arc::new(Self {
            handle,
            devices: devices.to_vec(),
            adapters,
        }))
    }

    /// Returns the number of enumerated devices visible to Unified Runtime.
    pub fn device_count() -> Result<usize, result::UrError> {
        ensure_loader_initialized()?;
        let adapters = get_process_adapters()?;

        if adapters.is_empty() {
            return Ok(0);
        }

        let mut count = 0usize;
        for &adapter in &adapters {
            let platforms = match unsafe { result::platform::get(adapter) } {
                Ok(platforms) => platforms,
                Err(error) => {
                    release_adapters(&adapters);
                    return Err(error);
                }
            };

            for platform in platforms {
                let devices = match unsafe {
                    result::device::get(platform, sys::ur_device_type_t::UR_DEVICE_TYPE_ALL)
                } {
                    Ok(devices) => devices,
                    Err(error) => {
                        release_adapters(&adapters);
                        return Err(error);
                    }
                };

                count += devices.len();
            }
        }

        release_adapters(&adapters);
        Ok(count)
    }

    /// Returns the device handles associated with this context.
    pub fn devices(&self) -> &[sys::ur_device_handle_t] {
        &self.devices
    }

    /// Returns the underlying Unified Runtime context handle.
    pub fn handle(&self) -> sys::ur_context_handle_t {
        self.handle
    }

    /// Returns the native handle for this context.
    pub fn native_handle(&self) -> Result<sys::ur_native_handle_t, result::UrError> {
        let mut native_handle = 0;
        unsafe {
            sys::urContextGetNativeHandle(self.handle, &mut native_handle).result()?;
        }
        Ok(native_handle)
    }

    /// Creates a queue on the first device associated with this context.
    pub fn new_queue(self: &Arc<Self>) -> Result<Arc<UrQueue>, result::UrError> {
        UrQueue::new(self)
    }

    /// Loads a program wrapper from SPIR-V IL bytes.
    pub fn load_program(
        self: &Arc<Self>,
        il: &[u8],
    ) -> Result<Arc<UrProgram>, result::UrError> {
        UrProgram::from_il(self, il)
    }
}

/// Represents a Unified Runtime queue tied to a context and a single device.
#[derive(Debug)]
pub struct UrQueue {
    pub(crate) handle: sys::ur_queue_handle_t,
    pub(crate) ctx: Arc<UrContext>,
    pub(crate) device: sys::ur_device_handle_t,
}

unsafe impl Send for UrQueue {}
unsafe impl Sync for UrQueue {}

impl Drop for UrQueue {
    fn drop(&mut self) {
        let handle = std::mem::replace(&mut self.handle, std::ptr::null_mut());
        if !handle.is_null() {
            let _ = unsafe { sys::urQueueRelease(handle).result() };
        }
    }
}

impl PartialEq for UrQueue {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle && self.ctx == other.ctx && self.device == other.device
    }
}

impl Eq for UrQueue {}

/// Safe event execution states for polling completion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UrEventStatus {
    Complete,
    Running,
    Submitted,
    Queued,
    Error,
}

impl UrEventStatus {
    fn from_raw(raw: u32) -> Result<Self, result::UrError> {
        match raw {
            value if value == sys::ur_event_status_t::UR_EVENT_STATUS_COMPLETE as u32 => {
                Ok(Self::Complete)
            }
            value if value == sys::ur_event_status_t::UR_EVENT_STATUS_RUNNING as u32 => {
                Ok(Self::Running)
            }
            value if value == sys::ur_event_status_t::UR_EVENT_STATUS_SUBMITTED as u32 => {
                Ok(Self::Submitted)
            }
            value if value == sys::ur_event_status_t::UR_EVENT_STATUS_QUEUED as u32 => {
                Ok(Self::Queued)
            }
            value if value == sys::ur_event_status_t::UR_EVENT_STATUS_ERROR as u32 => {
                Ok(Self::Error)
            }
            _ => Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_ENUMERATION,
            )),
        }
    }
}

/// Wrapper around `ur_event_handle_t`.
#[derive(Debug)]
pub struct UrEvent {
    pub(crate) handle: sys::ur_event_handle_t,
}

unsafe impl Send for UrEvent {}
unsafe impl Sync for UrEvent {}

impl Drop for UrEvent {
    fn drop(&mut self) {
        let handle = std::mem::replace(&mut self.handle, std::ptr::null_mut());
        if !handle.is_null() {
            let _ = unsafe { sys::urEventRelease(handle).result() };
        }
    }
}

impl PartialEq for UrEvent {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
    }
}

impl Eq for UrEvent {}

impl UrEvent {
    /// Returns the underlying Unified Runtime event handle.
    pub fn handle(&self) -> sys::ur_event_handle_t {
        self.handle
    }

    /// Waits until the event has completed.
    pub fn wait(&self) -> Result<(), result::UrError> {
        unsafe { sys::urEventWait(1, &self.handle).result() }
    }

    /// Waits until the event has completed.
    pub fn synchronize(&self) -> Result<(), result::UrError> {
        self.wait()
    }

    /// Returns the current execution status of the event without blocking.
    pub fn status(&self) -> Result<UrEventStatus, result::UrError> {
        let mut status = 0u32;
        unsafe {
            sys::urEventGetInfo(
                self.handle,
                sys::ur_event_info_t::UR_EVENT_INFO_COMMAND_EXECUTION_STATUS,
                std::mem::size_of::<u32>(),
                (&mut status as *mut u32).cast::<c_void>(),
                std::ptr::null_mut(),
            )
            .result()?;
        }
        UrEventStatus::from_raw(status)
    }

    /// Polls whether the event has completed.
    pub fn is_complete(&self) -> Result<bool, result::UrError> {
        Ok(self.status()? == UrEventStatus::Complete)
    }

    /// Poll-style alias for checking completion without blocking.
    pub fn query(&self) -> Result<bool, result::UrError> {
        self.is_complete()
    }

    /// Returns the native handle for this event.
    pub fn native_handle(&self) -> Result<sys::ur_native_handle_t, result::UrError> {
        let mut native_handle = 0;
        unsafe {
            sys::urEventGetNativeHandle(self.handle, &mut native_handle).result()?;
        }
        Ok(native_handle)
    }
}

impl UrQueue {
    /// Creates a queue on the first device associated with the given context.
    pub fn new(ctx: &Arc<UrContext>) -> Result<Arc<Self>, result::UrError> {
        let Some(&device) = ctx.devices().first() else {
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_DEVICE,
            ));
        };

        let mut handle = std::ptr::null_mut();
        unsafe {
            sys::urQueueCreate(ctx.handle(), device, std::ptr::null(), &mut handle).result()?;
        }

        Ok(Arc::new(Self {
            handle,
            ctx: Arc::clone(ctx),
            device,
        }))
    }

    /// Returns the underlying Unified Runtime queue handle.
    pub fn handle(&self) -> sys::ur_queue_handle_t {
        self.handle
    }

    /// Returns the context this queue belongs to.
    pub fn context(&self) -> &Arc<UrContext> {
        &self.ctx
    }

    /// Synchronizes the queue.
    pub fn synchronize(&self) -> Result<(), result::UrError> {
        unsafe { sys::urQueueFinish(self.handle).result() }
    }

    /// Records an event that becomes ready after all previously enqueued work on the queue.
    pub fn record_event(&self) -> Result<UrEvent, result::UrError> {
        self.enqueue_barrier(&[])
    }

    /// Allocates uninitialized USM memory for `len` elements of `T`.
    ///
    /// # Safety
    /// The returned memory is uninitialized.
    pub unsafe fn alloc<T>(
        self: &Arc<Self>,
        len: usize,
    ) -> Result<UrDeviceSlice<T>, result::UrError> {
        let mut ptr = std::ptr::null_mut();
        let desc = sys::ur_usm_desc_t {
            stype: sys::ur_structure_type_t::UR_STRUCTURE_TYPE_USM_DESC,
            pNext: std::ptr::null(),
            hints: 0,
            align: 0,
        };
        let size = len
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ))?;

        unsafe {
            sys::urUSMSharedAlloc(
                self.ctx.handle(),
                self.device,
                &desc,
                std::ptr::null_mut(),
                size,
                &mut ptr,
            )
            .result()?;
        }

        Ok(UrDeviceSlice {
            ptr: ptr.cast::<T>(),
            len,
            queue: Arc::clone(self),
            marker: PhantomData,
        })
    }

    /// Allocates USM memory for `len` elements of `T` and fills it with zero bytes.
    pub fn alloc_zeros<T: ValidAsZeroBits>(
        self: &Arc<Self>,
        len: usize,
    ) -> Result<UrDeviceSlice<T>, result::UrError> {
        let dst = unsafe { self.alloc::<T>(len) }?;
        let pattern = [0u8];
        let size = len
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ))?;

        self.enqueue_fill(dst.ptr.cast::<c_void>(), &pattern, size)?;

        Ok(dst)
    }

    /// Allocates device memory and copies the host slice into it.
    pub fn clone_htod<T: DeviceRepr>(
        self: &Arc<Self>,
        src: &[T],
    ) -> Result<UrDeviceSlice<T>, result::UrError> {
        let dst = unsafe { self.alloc::<T>(src.len()) }?;
        self.memcpy_htod(src, &dst)?;

        Ok(dst)
    }

    /// Copies a host slice into an existing USM allocation.
    pub fn memcpy_htod<T: DeviceRepr>(
        &self,
        src: &[T],
        dst: &UrDeviceSlice<T>,
    ) -> Result<(), result::UrError> {
        if src.len() != dst.len {
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ));
        }

        self.enqueue_memcpy(
            dst.ptr.cast::<c_void>(),
            src.as_ptr().cast::<c_void>(),
            std::mem::size_of_val(src),
        )
    }

    /// Copies a host slice into an existing USM allocation asynchronously.
    ///
    /// # Safety
    /// The source host slice and destination allocation must remain valid until the returned
    /// event has completed.
    pub unsafe fn memcpy_htod_async<T: DeviceRepr>(
        &self,
        src: &[T],
        dst: &UrDeviceSlice<T>,
        wait_for: &[&UrEvent],
    ) -> Result<UrEvent, result::UrError> {
        if src.len() != dst.len {
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ));
        }

        self.enqueue_memcpy_async(
            dst.ptr.cast::<c_void>(),
            src.as_ptr().cast::<c_void>(),
            std::mem::size_of_val(src),
            wait_for,
        )
    }

    /// Copies one USM allocation into another existing USM allocation.
    pub fn memcpy_dtod<T>(
        &self,
        src: &UrDeviceSlice<T>,
        dst: &UrDeviceSlice<T>,
    ) -> Result<(), result::UrError> {
        if src.len != dst.len {
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ));
        }

        let size = src
            .len
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ))?;

        self.enqueue_memcpy(dst.ptr.cast::<c_void>(), src.ptr.cast::<c_void>(), size)
    }

    /// Copies one USM allocation into another existing USM allocation asynchronously.
    ///
    /// # Safety
    /// Both allocations must remain valid until the returned event has completed.
    pub unsafe fn memcpy_dtod_async<T>(
        &self,
        src: &UrDeviceSlice<T>,
        dst: &UrDeviceSlice<T>,
        wait_for: &[&UrEvent],
    ) -> Result<UrEvent, result::UrError> {
        if src.len != dst.len {
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ));
        }

        let size = src
            .len
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ))?;

        self.enqueue_memcpy_async(
            dst.ptr.cast::<c_void>(),
            src.ptr.cast::<c_void>(),
            size,
            wait_for,
        )
    }

    /// Copies a USM allocation into a preallocated host slice.
    pub fn memcpy_dtoh<T: Copy>(
        &self,
        src: &UrDeviceSlice<T>,
        dst: &mut [T],
    ) -> Result<(), result::UrError> {
        if src.len != dst.len() {
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ));
        }

        self.enqueue_memcpy(
            dst.as_mut_ptr().cast::<c_void>(),
            src.ptr.cast::<c_void>(),
            std::mem::size_of_val(dst),
        )
    }

    /// Copies a USM allocation into a preallocated host slice asynchronously.
    ///
    /// # Safety
    /// The source allocation and destination host slice must remain valid until the returned
    /// event has completed.
    pub unsafe fn memcpy_dtoh_async<T: Copy>(
        &self,
        src: &UrDeviceSlice<T>,
        dst: &mut [T],
        wait_for: &[&UrEvent],
    ) -> Result<UrEvent, result::UrError> {
        if src.len != dst.len() {
            return Err(result::UrError(
                sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
            ));
        }

        self.enqueue_memcpy_async(
            dst.as_mut_ptr().cast::<c_void>(),
            src.ptr.cast::<c_void>(),
            std::mem::size_of_val(dst),
            wait_for,
        )
    }

    /// Copies a USM allocation into a newly allocated host vector.
    pub fn clone_dtoh<T: Default + Copy>(
        &self,
        src: &UrDeviceSlice<T>,
    ) -> Result<Vec<T>, result::UrError> {
        let mut dst = vec![T::default(); src.len];
        self.memcpy_dtoh(src, &mut dst)?;
        Ok(dst)
    }

    fn enqueue_memcpy(
        &self,
        dst: *mut c_void,
        src: *const c_void,
        size: usize,
    ) -> Result<(), result::UrError> {
        unsafe {
            sys::urEnqueueUSMMemcpy(
                self.handle,
                true,
                dst,
                src.cast_mut(),
                size,
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
            )
            .result()
        }
    }

    fn enqueue_memcpy_async(
        &self,
        dst: *mut c_void,
        src: *const c_void,
        size: usize,
        wait_for: &[&UrEvent],
    ) -> Result<UrEvent, result::UrError> {
        let wait_list = event_wait_list(wait_for);
        let mut event = std::ptr::null_mut();

        unsafe {
            sys::urEnqueueUSMMemcpy(
                self.handle,
                false,
                dst,
                src.cast_mut(),
                size,
                wait_list.len() as u32,
                wait_list.as_ptr(),
                &mut event,
            )
            .result()?;
        }

        Ok(UrEvent { handle: event })
    }

    fn enqueue_fill(
        &self,
        dst: *mut c_void,
        pattern: &[u8],
        size: usize,
    ) -> Result<(), result::UrError> {
        unsafe {
            sys::urEnqueueUSMFill(
                self.handle,
                dst,
                pattern.len(),
                pattern.as_ptr().cast::<c_void>(),
                size,
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
            )
            .result()
        }
    }

    fn enqueue_barrier(&self, wait_for: &[&UrEvent]) -> Result<UrEvent, result::UrError> {
        let wait_list = event_wait_list(wait_for);
        let mut event = std::ptr::null_mut();

        unsafe {
            sys::urEnqueueEventsWaitWithBarrier(
                self.handle,
                wait_list.len() as u32,
                wait_list.as_ptr(),
                &mut event,
            )
            .result()?;
        }

        Ok(UrEvent { handle: event })
    }
}

/// Wrapper around `ur_program_handle_t`.
#[derive(Debug)]
pub struct UrProgram {
    pub(crate) handle: sys::ur_program_handle_t,
    pub(crate) ctx: Arc<UrContext>,
    pub(crate) device: sys::ur_device_handle_t,
}

unsafe impl Send for UrProgram {}
unsafe impl Sync for UrProgram {}

impl Drop for UrProgram {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            let _ = unsafe { sys::urProgramRelease(self.handle).result() };
            self.handle = std::ptr::null_mut();
        }
    }
}

impl UrProgram {
    /// Creates a program from SPIR-V IL bytes for the given context.
    pub fn from_il(ctx: &Arc<UrContext>, il: &[u8]) -> Result<Arc<Self>, result::UrError> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            sys::urProgramCreateWithIL(
                ctx.handle(),
                il.as_ptr().cast::<c_void>(),
                il.len(),
                std::ptr::null(),
                &mut handle,
            )
            .result()?;
        }

        Ok(Arc::new(Self {
            handle,
            ctx: Arc::clone(ctx),
            device: ctx.devices()[0],
        }))
    }

    /// Builds the program for the device associated with its context.
    pub fn build(&self) -> Result<(), result::UrError> {
        unsafe { sys::urProgramBuild(self.ctx.handle(), self.handle, std::ptr::null()) }.result()
    }

    /// Returns the compiler build log for this program.
    pub fn build_log(&self) -> Result<String, result::UrError> {
        self.get_build_info_string(sys::ur_program_build_info_t::UR_PROGRAM_BUILD_INFO_LOG)
    }

    /// Creates a kernel wrapper by name from this program.
    pub fn create_kernel(self: &Arc<Self>, name: &str) -> Result<UrKernel, result::UrError> {
        let mut handle = std::ptr::null_mut();
        let name = CString::new(name).expect("kernel name must not contain NUL");
        unsafe {
            sys::urKernelCreate(self.handle, name.as_ptr(), &mut handle).result()?;
        }
        Ok(UrKernel {
            handle,
            program: Arc::clone(self),
        })
    }

    fn get_build_info_string(
        &self,
        info: sys::ur_program_build_info_t,
    ) -> Result<String, result::UrError> {
        let mut size = 0usize;
        unsafe {
            sys::urProgramGetBuildInfo(
                self.handle,
                self.device,
                info,
                0,
                std::ptr::null_mut(),
                &mut size,
            )
            .result()?;
        }
        if size == 0 {
            return Ok(String::new());
        }

        let mut bytes = vec![0u8; size];
        unsafe {
            sys::urProgramGetBuildInfo(
                self.handle,
                self.device,
                info,
                bytes.len(),
                bytes.as_mut_ptr().cast::<c_void>(),
                std::ptr::null_mut(),
            )
            .result()?;
        }

        let string = String::from_utf8_lossy(&bytes);
        Ok(string.trim_end_matches('\0').to_string())
    }
}

/// Wrapper around `ur_kernel_handle_t`.
#[derive(Debug)]
pub struct UrKernel {
    pub(crate) handle: sys::ur_kernel_handle_t,
    #[allow(unused)]
    pub(crate) program: Arc<UrProgram>,
}

unsafe impl Send for UrKernel {}
unsafe impl Sync for UrKernel {}

impl Drop for UrKernel {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            let _ = unsafe { sys::urKernelRelease(self.handle).result() };
            self.handle = std::ptr::null_mut();
        }
    }
}

impl UrKernel {
    pub unsafe fn set_arg_pointer(
        &self,
        index: u32,
        pointer: *const c_void,
    ) -> Result<(), result::UrError> {
        unsafe {
            sys::urKernelSetArgPointer(self.handle, index, std::ptr::null(), pointer).result()
        }
    }

    pub unsafe fn set_arg_value<T: DeviceRepr>(
        &self,
        index: u32,
        value: &T,
    ) -> Result<(), result::UrError> {
        unsafe {
            sys::urKernelSetArgValue(
                self.handle,
                index,
                std::mem::size_of::<T>(),
                std::ptr::null(),
                (value as *const T).cast::<c_void>(),
            )
            .result()
        }
    }

    pub fn launch(
        &self,
        queue: &Arc<UrQueue>,
        global_work_items: usize,
    ) -> Result<(), result::UrError> {
        self.enqueue_launch(queue, global_work_items, &[]).map(|_| ())
    }

    pub fn launch_async(
        &self,
        queue: &Arc<UrQueue>,
        global_work_items: usize,
        wait_for: &[&UrEvent],
    ) -> Result<UrEvent, result::UrError> {
        self.enqueue_launch(queue, global_work_items, wait_for)
    }

    fn enqueue_launch(
        &self,
        queue: &Arc<UrQueue>,
        global_work_items: usize,
        wait_for: &[&UrEvent],
    ) -> Result<UrEvent, result::UrError> {
        let global = [global_work_items];
        let wait_list = event_wait_list(wait_for);
        let mut event = std::ptr::null_mut();

        unsafe {
            sys::urEnqueueKernelLaunch(
                queue.handle(),
                self.handle,
                1,
                std::ptr::null(),
                global.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                wait_list.len() as u32,
                wait_list.as_ptr(),
                &mut event,
            )
            .result()?;
        }

        Ok(UrEvent { handle: event })
    }
}

/// A device allocation owned by a [UrQueue].
#[derive(Debug)]
pub struct UrDeviceSlice<T> {
    pub(crate) ptr: *mut T,
    pub(crate) len: usize,
    pub(crate) queue: Arc<UrQueue>,
    pub(crate) marker: PhantomData<T>,
}

unsafe impl<T: Send> Send for UrDeviceSlice<T> {}
unsafe impl<T: Sync> Sync for UrDeviceSlice<T> {}

impl<T> Drop for UrDeviceSlice<T> {
    fn drop(&mut self) {
        if !self.ptr.is_null() {
            let _ = unsafe {
                sys::urUSMFree(self.queue.ctx.handle(), self.ptr.cast::<std::ffi::c_void>())
                    .result()
            };
            self.ptr = std::ptr::null_mut();
        }
    }
}

impl<T> UrDeviceSlice<T> {
    /// Returns the number of elements in the allocation.
    pub fn len(&self) -> usize {
        self.len
    }

    /// Returns true if the allocation is empty.
    pub fn is_empty(&self) -> bool {
        self.len == 0
    }

    /// Returns the raw device pointer.
    pub fn as_mut_ptr(&self) -> *mut T {
        self.ptr
    }

    /// Returns the queue that owns this allocation.
    pub fn queue(&self) -> &Arc<UrQueue> {
        &self.queue
    }
}

impl<T> DevicePtr<T> for UrDeviceSlice<T> {
    fn device_ptr(&self) -> *const T {
        self.ptr.cast_const()
    }
}

impl<T> DevicePtrMut<T> for UrDeviceSlice<T> {
    fn device_ptr_mut(&mut self) -> *mut T {
        self.ptr
    }
}

fn enumerate_devices_for_adapters(
    adapters: &[sys::ur_adapter_handle_t],
) -> Result<Vec<sys::ur_device_handle_t>, result::UrError> {
    let mut devices = Vec::new();

    for &adapter in adapters {
        let platforms = match unsafe { result::platform::get(adapter) } {
            Ok(platforms) => platforms,
            Err(error) => {
                return Err(error);
            }
        };

        for platform in platforms {
            match unsafe {
                result::device::get(platform, sys::ur_device_type_t::UR_DEVICE_TYPE_ALL)
            } {
                Ok(platform_devices) => {
                    for device in platform_devices {
                        unsafe { sys::urDeviceRetain(device).result()? };
                        devices.push(device);
                    }
                }
                Err(error) => {
                    for &device in &devices {
                        let _ = unsafe { sys::urDeviceRelease(device).result() };
                    }
                    return Err(error);
                }
            }
        }
    }

    Ok(devices)
}

fn release_adapters(adapters: &[sys::ur_adapter_handle_t]) {
    for &adapter in adapters {
        let _ = unsafe { result::adapter::release(adapter) };
    }
}

fn event_wait_list(wait_for: &[&UrEvent]) -> Vec<sys::ur_event_handle_t> {
    wait_for.iter().map(|event| event.handle()).collect()
}
