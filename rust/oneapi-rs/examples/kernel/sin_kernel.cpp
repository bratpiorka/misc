//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

#include <sycl/sycl.hpp>

namespace syclext = sycl::ext::oneapi;
namespace syclexp = sycl::ext::oneapi::experimental;

extern "C" SYCL_EXT_ONEAPI_FUNCTION_PROPERTY(
    (syclexp::nd_range_kernel<1>)) void sin_kernel(float *out, const float *inp,
                                                   int numel) {
  const size_t i =
      syclext::this_work_item::get_nd_item<1>().get_global_linear_id();
  if (i < static_cast<size_t>(numel)) {
    out[i] = sycl::sin(inp[i]);
  }
}
