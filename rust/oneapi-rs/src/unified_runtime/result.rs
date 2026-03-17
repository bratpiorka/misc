//! A thin wrapper around [sys].
//!
//! Functions here return [Result], but most of the handle-based API remains unsafe.

use super::sys;
use core::ffi::c_void;

/// Wrapper around `ur_result_t`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct UnifiedRuntimeError(pub sys::ur_result_t);

impl sys::ur_result_t {
    #[inline]
    pub fn result(self) -> Result<(), UnifiedRuntimeError> {
        match self {
            sys::ur_result_t::UR_RESULT_SUCCESS => Ok(()),
            _ => Err(UnifiedRuntimeError(self)),
        }
    }
}

impl core::fmt::Debug for UnifiedRuntimeError {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("UnifiedRuntimeError")
            .field(&(self.0 as u32))
            .finish()
    }
}

impl std::fmt::Display for UnifiedRuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unified Runtime error code {}", self.0 as u32)
    }
}

impl std::error::Error for UnifiedRuntimeError {}

pub mod loader {
    //! Unified Runtime loader functions.

    use super::{UnifiedRuntimeError, sys};

    pub fn init(device_flags: sys::ur_device_init_flags_t) -> Result<(), UnifiedRuntimeError> {
        unsafe { sys::urLoaderInit(device_flags, core::ptr::null_mut()).result() }
    }

    pub fn tear_down() -> Result<(), UnifiedRuntimeError> {
        unsafe { sys::urLoaderTearDown().result() }
    }
}

pub mod adapter {
    //! Adapter discovery and lifetime helpers.

    use super::{UnifiedRuntimeError, sys};

    pub fn get() -> Result<Vec<sys::ur_adapter_handle_t>, UnifiedRuntimeError> {
        let mut count = 0u32;
        unsafe {
            sys::urAdapterGet(0, core::ptr::null_mut(), &mut count).result()?;
        }
        let mut adapters = vec![core::ptr::null_mut(); count as usize];
        unsafe {
            sys::urAdapterGet(count, adapters.as_mut_ptr(), core::ptr::null_mut()).result()?;
        }
        Ok(adapters)
    }

    /// # Safety
    /// Handles must be valid adapter handles returned by Unified Runtime.
    pub unsafe fn release(adapter: sys::ur_adapter_handle_t) -> Result<(), UnifiedRuntimeError> {
        unsafe { sys::urAdapterRelease(adapter).result() }
    }
}

pub mod platform {
    //! Platform enumeration helpers.

    use super::{UnifiedRuntimeError, get_info_string, sys};

    /// # Safety
    /// `adapter` must be a valid adapter handle.
    pub unsafe fn get(
        adapter: sys::ur_adapter_handle_t,
    ) -> Result<Vec<sys::ur_platform_handle_t>, UnifiedRuntimeError> {
        let mut count = 0u32;
        unsafe {
            sys::urPlatformGet(adapter, 0, core::ptr::null_mut(), &mut count).result()?;
        }
        let mut platforms = vec![core::ptr::null_mut(); count as usize];
        unsafe {
            sys::urPlatformGet(adapter, count, platforms.as_mut_ptr(), core::ptr::null_mut())
                .result()?;
        }
        Ok(platforms)
    }

    /// # Safety
    /// `platform` must be a valid platform handle.
    pub unsafe fn name(platform: sys::ur_platform_handle_t) -> Result<String, UnifiedRuntimeError> {
        unsafe {
            get_info_string(
                |prop_size, prop_value, prop_size_ret| {
                    sys::urPlatformGetInfo(
                        platform,
                        sys::ur_platform_info_t::UR_PLATFORM_INFO_NAME,
                        prop_size,
                        prop_value,
                        prop_size_ret,
                    )
                },
            )
        }
    }

}

pub mod device {
    //! Device enumeration helpers.

    use super::{UnifiedRuntimeError, get_info_string, sys};

    /// # Safety
    /// `platform` must be a valid platform handle.
    pub unsafe fn get(
        platform: sys::ur_platform_handle_t,
        device_type: sys::ur_device_type_t,
    ) -> Result<Vec<sys::ur_device_handle_t>, UnifiedRuntimeError> {
        let mut count = 0u32;
        unsafe {
            sys::urDeviceGet(platform, device_type, 0, core::ptr::null_mut(), &mut count)
                .result()?;
        }
        let mut devices = vec![core::ptr::null_mut(); count as usize];
        unsafe {
            sys::urDeviceGet(
                platform,
                device_type,
                count,
                devices.as_mut_ptr(),
                core::ptr::null_mut(),
            )
            .result()?;
        }
        Ok(devices)
    }

    /// # Safety
    /// `device` must be a valid device handle.
    pub unsafe fn name(device: sys::ur_device_handle_t) -> Result<String, UnifiedRuntimeError> {
        unsafe {
            get_info_string(
                |prop_size, prop_value, prop_size_ret| {
                    sys::urDeviceGetInfo(
                        device,
                        sys::ur_device_info_t::UR_DEVICE_INFO_NAME,
                        prop_size,
                        prop_value,
                        prop_size_ret,
                    )
                },
            )
        }
    }
}

fn get_info_string(
    mut get_info: impl FnMut(usize, *mut c_void, *mut usize) -> sys::ur_result_t,
) -> Result<String, UnifiedRuntimeError> {
    let mut size = 0usize;
    get_info(0, core::ptr::null_mut(), &mut size).result()?;

    let mut buffer = vec![0u8; size];
    get_info(
        buffer.len(),
        buffer.as_mut_ptr().cast::<c_void>(),
        core::ptr::null_mut(),
    )
    .result()?;

    if buffer.last() == Some(&0) {
        buffer.pop();
    }

    Ok(String::from_utf8_lossy(&buffer).into_owned())
}