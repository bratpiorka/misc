use crate::unified_runtime::{result, sys};

use std::{marker::PhantomData, sync::Arc};

/// Represents a Unified Runtime context over one or more devices.
///
/// This is the entry point for owning a `ur_context_handle_t` safely in Rust.
#[derive(Debug)]
pub struct Context {
    pub(crate) handle: sys::ur_context_handle_t,
    devices: Vec<sys::ur_device_handle_t>,
    adapters: Vec<sys::ur_adapter_handle_t>,
}

unsafe impl Send for Context {}
unsafe impl Sync for Context {}

impl Drop for Context {
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

impl PartialEq for Context {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle
            && self.devices == other.devices
            && self.adapters == other.adapters
    }
}

impl Eq for Context {}

impl Context {
    /// Creates a context from the enumerated device ordinal across all discovered adapters/platforms.
    pub fn new(ordinal: usize) -> Result<Arc<Self>, result::UnifiedRuntimeError> {
        result::loader::init(sys::ur_device_init_flag_t::UR_DEVICE_INIT_FLAG_GPU as _)?;

        let adapters = result::adapter::get()?;
        let devices = enumerate_devices_for_adapters(&adapters)?;
        let Some(&device) = devices.get(ordinal) else {
            release_adapters(&adapters);
            for device in devices {
                let _ = unsafe { sys::urDeviceRelease(device).result() };
            }
            return Err(result::UnifiedRuntimeError(
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
    ) -> Result<Arc<Self>, result::UnifiedRuntimeError> {
        unsafe { Self::from_devices_with_adapters(&[device], Vec::new()) }
    }

    /// Creates a context from a slice of device handles.
    ///
    /// # Safety
    /// All device handles must be valid and belong to compatible Unified Runtime backends.
    pub unsafe fn from_devices(
        devices: &[sys::ur_device_handle_t],
    ) -> Result<Arc<Self>, result::UnifiedRuntimeError> {
        unsafe { Self::from_devices_with_adapters(devices, Vec::new()) }
    }

    unsafe fn from_devices_with_adapters(
        devices: &[sys::ur_device_handle_t],
        adapters: Vec<sys::ur_adapter_handle_t>,
    ) -> Result<Arc<Self>, result::UnifiedRuntimeError> {
        if devices.is_empty() {
            release_adapters(&adapters);
            return Err(result::UnifiedRuntimeError(
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
    pub fn device_count() -> Result<usize, result::UnifiedRuntimeError> {
        result::loader::init(sys::ur_device_init_flag_t::UR_DEVICE_INIT_FLAG_GPU as _)?;
        let devices = enumerate_devices()?;
        let count = devices.len();
        for device in devices {
            let _ = unsafe { sys::urDeviceRelease(device).result() };
        }
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
    pub fn native_handle(&self) -> Result<sys::ur_native_handle_t, result::UnifiedRuntimeError> {
        let mut native_handle = 0;
        unsafe {
            sys::urContextGetNativeHandle(self.handle, &mut native_handle).result()?;
        }
        Ok(native_handle)
    }

    /// Creates a queue on the first device associated with this context.
    pub fn new_queue(self: &Arc<Self>) -> Result<Arc<Queue>, result::UnifiedRuntimeError> {
        Queue::new(self)
    }
}

/// Represents a Unified Runtime queue tied to a context and a single device.
#[derive(Debug)]
pub struct Queue {
    pub(crate) handle: sys::ur_queue_handle_t,
    pub(crate) ctx: Arc<Context>,
    pub(crate) device: sys::ur_device_handle_t,
}

unsafe impl Send for Queue {}
unsafe impl Sync for Queue {}

impl Drop for Queue {
    fn drop(&mut self) {
        let handle = std::mem::replace(&mut self.handle, std::ptr::null_mut());
        if !handle.is_null() {
            let _ = unsafe { sys::urQueueRelease(handle).result() };
        }
    }
}

impl PartialEq for Queue {
    fn eq(&self, other: &Self) -> bool {
        self.handle == other.handle && self.ctx == other.ctx && self.device == other.device
    }
}

impl Eq for Queue {}

impl Queue {
    /// Creates a queue on the first device associated with the given context.
    pub fn new(ctx: &Arc<Context>) -> Result<Arc<Self>, result::UnifiedRuntimeError> {
        let Some(&device) = ctx.devices().first() else {
            return Err(result::UnifiedRuntimeError(
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
    pub fn context(&self) -> &Arc<Context> {
        &self.ctx
    }

    /// Synchronizes the queue.
    pub fn synchronize(&self) -> Result<(), result::UnifiedRuntimeError> {
        unsafe { sys::urQueueFinish(self.handle).result() }
    }

    /// Allocates uninitialized USM memory for `len` elements of `T`.
    ///
    /// # Safety
    /// The returned memory is uninitialized.
    pub unsafe fn alloc<T>(
        self: &Arc<Self>,
        len: usize,
    ) -> Result<UrDeviceSlice<T>, result::UnifiedRuntimeError> {
        let mut ptr = std::ptr::null_mut();
        let desc = sys::ur_usm_desc_t {
            stype: sys::ur_structure_type_t::UR_STRUCTURE_TYPE_USM_DESC,
            pNext: std::ptr::null(),
            hints: 0,
            align: 0,
        };
        let size = len
            .checked_mul(std::mem::size_of::<T>())
            .ok_or(result::UnifiedRuntimeError(
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

    /// Allocates device memory and copies the host slice into it.
    pub fn clone_htod<T: Copy>(
        self: &Arc<Self>,
        src: &[T],
    ) -> Result<UrDeviceSlice<T>, result::UnifiedRuntimeError> {
        let dst = unsafe { self.alloc::<T>(src.len()) }?;
        let size =
            src.len()
                .checked_mul(std::mem::size_of::<T>())
                .ok_or(result::UnifiedRuntimeError(
                    sys::ur_result_t::UR_RESULT_ERROR_INVALID_SIZE,
                ))?;

        unsafe {
            sys::urEnqueueUSMMemcpy(
                self.handle,
                true,
                dst.ptr.cast::<std::ffi::c_void>(),
                src.as_ptr().cast::<std::ffi::c_void>(),
                size,
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
            )
            .result()?;
        }

        Ok(dst)
    }
}

/// A device allocation owned by a [Queue].
#[derive(Debug)]
pub struct UrDeviceSlice<T> {
    pub(crate) ptr: *mut T,
    pub(crate) len: usize,
    pub(crate) queue: Arc<Queue>,
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
    pub fn queue(&self) -> &Arc<Queue> {
        &self.queue
    }
}

fn enumerate_devices() -> Result<Vec<sys::ur_device_handle_t>, result::UnifiedRuntimeError> {
    let adapters = result::adapter::get()?;
    let devices = enumerate_devices_for_adapters(&adapters);
    release_adapters(&adapters);
    devices
}

fn enumerate_devices_for_adapters(
    adapters: &[sys::ur_adapter_handle_t],
) -> Result<Vec<sys::ur_device_handle_t>, result::UnifiedRuntimeError> {
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
