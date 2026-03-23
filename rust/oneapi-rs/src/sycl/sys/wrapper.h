//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

#pragma once

#include <stddef.h>

#ifdef __cplusplus
extern "C" {
#endif

typedef struct sycl_rs_device_t sycl_rs_device_t;
typedef struct sycl_rs_context_t sycl_rs_context_t;
typedef struct sycl_rs_queue_t sycl_rs_queue_t;
typedef struct sycl_rs_event_t sycl_rs_event_t;
typedef struct sycl_rs_program_t sycl_rs_program_t;
typedef struct sycl_rs_kernel_t sycl_rs_kernel_t;

typedef enum sycl_rs_result_t {
    SYCL_RS_RESULT_SUCCESS = 0,
    SYCL_RS_RESULT_INVALID_ARGUMENT = 1,
    SYCL_RS_RESULT_OUT_OF_MEMORY = 2,
    SYCL_RS_RESULT_RUNTIME_ERROR = 3,
} sycl_rs_result_t;

typedef enum sycl_rs_alloc_kind_t {
    SYCL_RS_ALLOC_KIND_DEVICE = 0,
    SYCL_RS_ALLOC_KIND_SHARED = 1,
    SYCL_RS_ALLOC_KIND_HOST = 2,
} sycl_rs_alloc_kind_t;

typedef struct sycl_rs_raw_kernel_arg_t {
    const void *data;
    size_t size;
} sycl_rs_raw_kernel_arg_t;

const char *sycl_rs_last_error_message(void);

sycl_rs_result_t sycl_rs_device_count(size_t *out_count);
sycl_rs_result_t sycl_rs_device_create_with_index(size_t index,
                                                  sycl_rs_device_t **out_device);
sycl_rs_result_t sycl_rs_device_create_default(sycl_rs_device_t **out_device);
void sycl_rs_device_destroy(sycl_rs_device_t *device);

sycl_rs_result_t sycl_rs_context_create(
    const sycl_rs_device_t *device,
    sycl_rs_context_t **out_context
);
void sycl_rs_context_destroy(sycl_rs_context_t *context);

sycl_rs_result_t sycl_rs_queue_create(
    const sycl_rs_context_t *context,
    const sycl_rs_device_t *device,
    sycl_rs_queue_t **out_queue
);
void sycl_rs_queue_destroy(sycl_rs_queue_t *queue);

void sycl_rs_event_destroy(sycl_rs_event_t *event);
sycl_rs_result_t sycl_rs_event_wait(const sycl_rs_event_t *event);

sycl_rs_result_t sycl_rs_alloc(
    sycl_rs_queue_t *queue,
    sycl_rs_alloc_kind_t kind,
    size_t bytes,
    size_t alignment,
    void **out_ptr
);
sycl_rs_result_t sycl_rs_free(sycl_rs_queue_t *queue, void *ptr);
sycl_rs_result_t sycl_rs_memcpy(
    sycl_rs_queue_t *queue,
    void *dst,
    const void *src,
    size_t bytes
);
sycl_rs_result_t sycl_rs_memcpy_async(
    sycl_rs_queue_t *queue,
    void *dst,
    const void *src,
    size_t bytes,
    sycl_rs_event_t **out_event
);
sycl_rs_result_t sycl_rs_queue_wait(sycl_rs_queue_t *queue);

sycl_rs_result_t sycl_rs_program_build_from_source(
    const sycl_rs_context_t *context,
    const sycl_rs_device_t *device,
    const char *source,
    size_t source_len,
    const char *build_options,
    sycl_rs_program_t **out_program
);
const char *sycl_rs_program_last_log(const sycl_rs_program_t *program);
void sycl_rs_program_destroy(sycl_rs_program_t *program);

sycl_rs_result_t sycl_rs_program_get_kernel(
    const sycl_rs_program_t *program,
    const char *kernel_name,
    sycl_rs_kernel_t **out_kernel
);
void sycl_rs_kernel_destroy(sycl_rs_kernel_t *kernel);

sycl_rs_result_t sycl_rs_kernel_launch_1d(
    sycl_rs_queue_t *queue,
    const sycl_rs_kernel_t *kernel,
    size_t global_range,
    size_t local_range,
    const sycl_rs_raw_kernel_arg_t *args,
    size_t num_args
);

#ifdef __cplusplus
}
#endif
