//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

use std::{error::Error, io};

use oneapi_rs::sycl::{
    self,
    safe::{SyclContext, SyclKernelArg, SyclQueue},
};

const INIT_KERNEL_NAME: &str = "init_values";
const ADD_ONE_KERNEL_NAME: &str = "add_one";
const KERNEL_SOURCE: &str = include_str!("multi_gpu_kernels.cpp");

type ExampleResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> ExampleResult<()> {
    let device_count = SyclContext::device_count()?;
    println!("Discovered {device_count} SYCL device(s).");

    if device_count < 2 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "multi_gpu example requires at least 2 devices",
        )
        .into());
    }

    let queue1 = SyclQueue::new_for_device_ordinal(0)?;
    let queue2 = SyclQueue::new_for_device_ordinal(1)?;

    let program1 = queue1.load_program_from_source(KERNEL_SOURCE)?;
    let init_kernel = program1.create_kernel(INIT_KERNEL_NAME)?;

    let program2 = queue2.load_program_from_source(KERNEL_SOURCE)?;
    let add_one_kernel = program2.create_kernel(ADD_ONE_KERNEL_NAME)?;

    let size = 1 << 20;
    let element_count = i32::try_from(size).map_err(|_| {
        io::Error::new(io::ErrorKind::InvalidInput, "size does not fit in i32")
    })?;

    let mut src = unsafe { queue1.alloc_device::<u32>(size) }?;
    let init_args = [
        SyclKernelArg::device_ptr_mut(&mut src),
        SyclKernelArg::scalar(&element_count),
    ];
    unsafe {
        init_kernel.launch_1d(&queue1, size, 1, &init_args)?;
    }

    let mut staged = vec![0u32; size];
    let copy_to_host = sycl::memcpy(&queue1, &src, staged.as_mut_slice())?;
    copy_to_host.wait()?;

    let mut dst = queue2.alloc_zeros::<u32>(size)?;
    let copy_to_device = sycl::memcpy(&queue2, staged.as_slice(), &dst)?;
    copy_to_device.wait()?;

    let add_one_args = [
        SyclKernelArg::device_ptr_mut(&mut dst),
        SyclKernelArg::scalar(&element_count),
    ];
    unsafe {
        add_one_kernel.launch_1d(&queue2, size, 1, &add_one_args)?;
    }

    let mut result = vec![0u32; size];
    let copy_back = sycl::memcpy(&queue2, &dst, result.as_mut_slice())?;
    copy_back.wait()?;

    let expected_source: Vec<u32> = (0..size as u32).collect();
    let expected_result: Vec<u32> = (1..=size as u32).collect();

    assert_eq!(staged, expected_source);
    assert_eq!(result, expected_result);

    println!(
        "Initialized {} values on device 0, staged them through host memory, and incremented them on device 1.",
        size
    );

    Ok(())
}