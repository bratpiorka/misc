//===----------------------------------------------------------------------===//
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
