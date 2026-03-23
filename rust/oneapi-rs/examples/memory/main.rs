//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

use oneapi_rs::sycl::{self, result::SyclError, safe::SyclQueue};

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), SyclError> {
    let queue = SyclQueue::new_default()?;

    let input = [10u32, 20, 30, 40];
    let shared = unsafe { queue.alloc_shared::<u32>(input.len()) }?;

    let copy_to_shared = sycl::memcpy(&queue, &input, &shared)?;
    copy_to_shared.wait()?;

    let mut output = vec![0u32; input.len()];
    let copy_to_host = sycl::memcpy(&queue, &shared, output.as_mut_slice())?;
    copy_to_host.wait()?;

    assert_eq!(output, input);

    println!(
        "Allocated {} shared u32 values and copied them back successfully with async events.",
        output.len()
    );

    Ok(())
}