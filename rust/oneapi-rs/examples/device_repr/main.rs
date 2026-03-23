//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

use oneapi_rs::sycl::{self, result::SyclError, safe::{SyclKernelArg, SyclQueue}};

const KERNEL_NAME: &str = "check_device_repr";
const KERNEL_SOURCE: &str = include_str!("device_repr_kernel.cpp");

/// Match the kernel-side layout exactly so the by-value kernel argument is well defined.
#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct MyCoolRustStruct {
    a: f32,
    b: f64,
    c: u32,
    d: usize,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), SyclError> {
    let queue = SyclQueue::new_default()?;
    let program = queue.load_program_from_source(KERNEL_SOURCE)?;
    let kernel = program.create_kernel(KERNEL_NAME)?;

    let thing = MyCoolRustStruct {
        a: 1.0,
        b: 2.34,
        c: 57,
        d: 420,
    };

    let mut device_status = queue.alloc_zeros::<u64>(1)?;
    let args = [
        SyclKernelArg::scalar(&thing),
        SyclKernelArg::buffer_mut(&mut device_status),
    ];

    unsafe {
        kernel.launch_1d(&queue, 1, 1, &args)?;
    }

    let mut status = [0u64; 1];
    sycl::memcpy_sync(&queue, &device_status, &mut status)?;

    assert_eq!(status[0], 1, "device-side struct validation failed");

    println!("Passed a #[repr(C)] Rust struct by value to a SYCL kernel successfully.");

    Ok(())
}