//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

use oneapi_rs::sycl::{
    self, 
    result::SyclError, 
    safe::{
        DevicePtr, 
        DevicePtrMut, 
        SyclKernelArg, 
        SyclQueue
    }
};

const KERNEL_NAME: &str = "sin_kernel";
const KERNEL_SOURCE: &str = include_str!("sin_kernel.cpp");

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

    let host_input = [0.0f32, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
    let device_input = unsafe { queue.alloc_device::<f32>(host_input.len()) }?;
    sycl::memcpy_sync(&queue, &host_input, &device_input)?;
    let mut device_output = unsafe { queue.alloc_device::<f32>(host_input.len()) }?;
    let element_count = host_input.len() as i32;

    let args = [
        SyclKernelArg::ptr_mut(device_output.device_ptr_mut()),
        SyclKernelArg::ptr(device_input.device_ptr()),
        SyclKernelArg::scalar(&element_count),
    ];

    unsafe {
        kernel.launch_1d(&queue, host_input.len(), 1, &args)?;
    }

    let mut host_output = vec![0.0f32; host_input.len()];
    sycl::memcpy_sync(&queue, &device_output, host_output.as_mut_slice())?;

    for (input, output) in host_input.iter().zip(&host_output) {
        println!("sin({input:.3}) = {output:.6}");
    }

    let max_error = host_input
        .iter()
        .zip(&host_output)
        .map(|(input, output)| (input.sin() - output).abs())
        .fold(0.0f32, f32::max);

    println!("Max abs error: {max_error:.8}");

    Ok(())
}
