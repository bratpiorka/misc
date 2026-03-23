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

struct MyCoolStruct {
  float a;
  double b;
  unsigned int c;
  size_t d;
};

extern "C" SYCL_EXT_ONEAPI_FUNCTION_PROPERTY(
    (syclexp::nd_range_kernel<1>)) void check_device_repr(MyCoolStruct thing,
                                                          unsigned long long
                                                              *status) {
  const size_t i =
      syclext::this_work_item::get_nd_item<1>().get_global_linear_id();
  if (i != 0) {
    return;
  }

  status[0] = (thing.a == 1.0f && thing.b == 2.34 && thing.c == 57u &&
               thing.d == static_cast<size_t>(420))
                  ? 1ull
                  : 0ull;
}