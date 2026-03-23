//===----------------------------------------------------------------------===//
//
// Part of the LLVM Project, under the Apache License v2.0 with LLVM Exceptions.
// See https://llvm.org/LICENSE.txt for license information.
// SPDX-License-Identifier: Apache-2.0 WITH LLVM-exception
//
//===----------------------------------------------------------------------===//

#include "wrapper.h"

#include <sycl/sycl.hpp>

#include <exception>
#include <new>
#include <stdexcept>
#include <string>
#include <utility>
#include <vector>

namespace syclexp = sycl::ext::oneapi::experimental;

struct sycl_rs_device_t {
    explicit sycl_rs_device_t(sycl::device device) : value(std::move(device)) {}

    sycl::device value;
};

struct sycl_rs_context_t {
    explicit sycl_rs_context_t(sycl::context context) : value(std::move(context)) {}

    sycl::context value;
};

struct sycl_rs_queue_t {
    explicit sycl_rs_queue_t(sycl::queue queue) : value(std::move(queue)) {}

    sycl::queue value;
};

struct sycl_rs_event_t {
    explicit sycl_rs_event_t(sycl::event event) : value(std::move(event)) {}

    sycl::event value;
};

struct sycl_rs_program_t {
    sycl_rs_program_t(sycl::kernel_bundle<sycl::bundle_state::executable> bundle, std::string log)
        : value(std::move(bundle)), build_log(std::move(log)) {}

    sycl::kernel_bundle<sycl::bundle_state::executable> value;
    std::string build_log;
};

struct sycl_rs_kernel_t {
    explicit sycl_rs_kernel_t(sycl::kernel kernel) : value(std::move(kernel)) {}

    sycl::kernel value;
};

namespace {

thread_local std::string g_last_error;

void set_error(const char *message) {
    g_last_error = message != nullptr ? message : "unknown SYCL error";
}

void clear_error() {
    g_last_error.clear();
}

std::vector<sycl::device> enumerate_devices() {
    return sycl::device::get_devices(sycl::info::device_type::all);
}

template <typename Func>
sycl_rs_result_t with_exceptions(Func &&func) {
    try {
        func();
        clear_error();
        return SYCL_RS_RESULT_SUCCESS;
    } catch (const sycl::exception &error) {
        set_error(error.what());
        return SYCL_RS_RESULT_RUNTIME_ERROR;
    } catch (const std::bad_alloc &error) {
        set_error(error.what());
        return SYCL_RS_RESULT_OUT_OF_MEMORY;
    } catch (const std::exception &error) {
        set_error(error.what());
        return SYCL_RS_RESULT_RUNTIME_ERROR;
    } catch (...) {
        set_error("unknown non-standard exception");
        return SYCL_RS_RESULT_RUNTIME_ERROR;
    }
}

sycl_rs_result_t validate_output_pointer(const void *out_ptr) {
    if (out_ptr == nullptr) {
        set_error("output pointer must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return SYCL_RS_RESULT_SUCCESS;
}

}  // namespace

extern "C" {

const char *sycl_rs_last_error_message(void) {
    return g_last_error.c_str();
}

sycl_rs_result_t sycl_rs_device_count(size_t *out_count) {
    if (auto status = validate_output_pointer(out_count); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        *out_count = enumerate_devices().size();
    });
}

sycl_rs_result_t sycl_rs_device_create_with_index(size_t index, sycl_rs_device_t **out_device) {
    if (auto status = validate_output_pointer(out_device); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        auto devices = enumerate_devices();
        if (index >= devices.size()) {
            set_error("device index out of range");
            throw std::invalid_argument("device index out of range");
        }

        *out_device = new sycl_rs_device_t(devices[index]);
    });
}

sycl_rs_result_t sycl_rs_device_create_default(sycl_rs_device_t **out_device) {
    if (auto status = validate_output_pointer(out_device); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        *out_device = new sycl_rs_device_t(sycl::device(sycl::default_selector_v));
    });
}

void sycl_rs_device_destroy(sycl_rs_device_t *device) {
    delete device;
}

sycl_rs_result_t sycl_rs_context_create(
    const sycl_rs_device_t *device,
    sycl_rs_context_t **out_context
) {
    if (device == nullptr) {
        set_error("device must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_context); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        *out_context = new sycl_rs_context_t(sycl::context(device->value));
    });
}

void sycl_rs_context_destroy(sycl_rs_context_t *context) {
    delete context;
}

sycl_rs_result_t sycl_rs_queue_create(
    const sycl_rs_context_t *context,
    const sycl_rs_device_t *device,
    sycl_rs_queue_t **out_queue
) {
    if (context == nullptr) {
        set_error("context must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (device == nullptr) {
        set_error("device must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_queue); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        *out_queue = new sycl_rs_queue_t(sycl::queue(context->value, device->value));
    });
}

void sycl_rs_queue_destroy(sycl_rs_queue_t *queue) {
    delete queue;
}

void sycl_rs_event_destroy(sycl_rs_event_t *event) {
    delete event;
}

sycl_rs_result_t sycl_rs_event_wait(const sycl_rs_event_t *event) {
    if (event == nullptr) {
        set_error("event must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] {
        auto queued_event = event->value;
        queued_event.wait();
    });
}

sycl_rs_result_t sycl_rs_alloc(
    sycl_rs_queue_t *queue,
    sycl_rs_alloc_kind_t kind,
    size_t bytes,
    size_t alignment,
    void **out_ptr
) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_ptr); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        void *ptr = nullptr;
        switch (kind) {
            case SYCL_RS_ALLOC_KIND_DEVICE:
                ptr = alignment > 0 ? sycl::aligned_alloc_device(alignment, bytes, queue->value)
                                    : sycl::malloc_device(bytes, queue->value);
                break;
            case SYCL_RS_ALLOC_KIND_SHARED:
                ptr = alignment > 0 ? sycl::aligned_alloc_shared(alignment, bytes, queue->value)
                                    : sycl::malloc_shared(bytes, queue->value);
                break;
            case SYCL_RS_ALLOC_KIND_HOST:
                ptr = alignment > 0 ? sycl::aligned_alloc_host(alignment, bytes, queue->value)
                                    : sycl::malloc_host(bytes, queue->value);
                break;
            default:
                set_error("unknown allocation kind");
                throw std::invalid_argument("unknown allocation kind");
        }

        if (bytes > 0 && ptr == nullptr) {
            set_error("SYCL allocation returned null");
            throw std::bad_alloc();
        }

        *out_ptr = ptr;
    });
}

sycl_rs_result_t sycl_rs_free(sycl_rs_queue_t *queue, void *ptr) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] { sycl::free(ptr, queue->value); });
}

sycl_rs_result_t sycl_rs_memset(sycl_rs_queue_t *queue, void *dst, int value,
                                size_t bytes) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (bytes > 0 && dst == nullptr) {
        set_error("memset destination must not be null when bytes > 0");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] { queue->value.memset(dst, value, bytes).wait(); });
}

sycl_rs_result_t sycl_rs_memcpy(
    sycl_rs_queue_t *queue,
    void *dst,
    const void *src,
    size_t bytes
) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (bytes > 0 && (dst == nullptr || src == nullptr)) {
        set_error("memcpy source and destination must not be null when bytes > 0");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] { queue->value.memcpy(dst, src, bytes).wait(); });
}

sycl_rs_result_t sycl_rs_memcpy_async(
    sycl_rs_queue_t *queue,
    void *dst,
    const void *src,
    size_t bytes,
    sycl_rs_event_t **out_event
) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (bytes > 0 && (dst == nullptr || src == nullptr)) {
        set_error("memcpy source and destination must not be null when bytes > 0");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_event); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        sycl::event event = queue->value.memcpy(dst, src, bytes);
        *out_event = new sycl_rs_event_t(std::move(event));
    });
}

sycl_rs_result_t sycl_rs_queue_wait(sycl_rs_queue_t *queue) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] { queue->value.wait(); });
}

sycl_rs_result_t sycl_rs_program_build_from_source(
    const sycl_rs_context_t *context,
    const sycl_rs_device_t *device,
    const char *source,
    size_t source_len,
    const char *build_options,
    sycl_rs_program_t **out_program
) {
    if (context == nullptr) {
        set_error("context must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (device == nullptr) {
        set_error("device must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (source == nullptr) {
        set_error("source must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_program); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        auto build_device = device->value;
        if (!build_device.ext_oneapi_can_build(syclexp::source_language::sycl)) {
            set_error("device does not support SYCL runtime compilation");
            throw std::runtime_error("device does not support SYCL runtime compilation");
        }

        auto source_bundle = syclexp::create_kernel_bundle_from_source(
            context->value,
            syclexp::source_language::sycl,
            std::string(source, source_len));

        std::string build_log;
        std::vector<sycl::device> devices{build_device};
        auto executable_bundle =
            (build_options != nullptr && build_options[0] != '\0')
                ? syclexp::build(
                      source_bundle,
                      devices,
                      syclexp::properties{
                          syclexp::build_options{std::string(build_options)},
                          syclexp::save_log(&build_log),
                      })
                : syclexp::build(
                      source_bundle,
                      devices,
                      syclexp::properties{syclexp::save_log(&build_log)});

        *out_program = new sycl_rs_program_t(std::move(executable_bundle), std::move(build_log));
    });
}

const char *sycl_rs_program_last_log(const sycl_rs_program_t *program) {
    return program == nullptr ? nullptr : program->build_log.c_str();
}

void sycl_rs_program_destroy(sycl_rs_program_t *program) {
    delete program;
}

sycl_rs_result_t sycl_rs_program_get_kernel(
    const sycl_rs_program_t *program,
    const char *kernel_name,
    sycl_rs_kernel_t **out_kernel
) {
    if (program == nullptr) {
        set_error("program must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (kernel_name == nullptr) {
        set_error("kernel_name must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (auto status = validate_output_pointer(out_kernel); status != SYCL_RS_RESULT_SUCCESS) {
        return status;
    }

    return with_exceptions([&] {
        auto bundle = program->value;
        *out_kernel = new sycl_rs_kernel_t(bundle.ext_oneapi_get_kernel(std::string(kernel_name)));
    });
}

void sycl_rs_kernel_destroy(sycl_rs_kernel_t *kernel) {
    delete kernel;
}

sycl_rs_result_t sycl_rs_kernel_launch_1d(
    sycl_rs_queue_t *queue,
    const sycl_rs_kernel_t *kernel,
    size_t global_range,
    size_t local_range,
    const sycl_rs_raw_kernel_arg_t *args,
    size_t num_args
) {
    if (queue == nullptr) {
        set_error("queue must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (kernel == nullptr) {
        set_error("kernel must not be null");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (global_range == 0 || local_range == 0) {
        set_error("global_range and local_range must be greater than zero");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (global_range % local_range != 0) {
        set_error("global_range must be divisible by local_range");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }
    if (num_args > 0 && args == nullptr) {
        set_error("args must not be null when num_args > 0");
        return SYCL_RS_RESULT_INVALID_ARGUMENT;
    }

    return with_exceptions([&] {
        queue->value
            .submit([&](sycl::handler &cgh) {
                for (size_t index = 0; index < num_args; ++index) {
                    cgh.set_arg(
                        static_cast<int>(index),
                        syclexp::raw_kernel_arg(args[index].data, args[index].size));
                }
                cgh.parallel_for(
                    sycl::nd_range<1>{{global_range}, {local_range}},
                    kernel->value);
            })
            .wait();
    });
}

}  // extern "C"
