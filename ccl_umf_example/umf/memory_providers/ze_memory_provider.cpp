/*
    Copyright (c) 2023 Intel Corporation
    Licensed under the Apache License, Version 2.0 (the "License");
    you may not use this file except in compliance with the License.
    You may obtain a copy of the License at
        http://www.apache.org/licenses/LICENSE-2.0
    Unless required by applicable law or agreed to in writing, software
    distributed under the License is distributed on an "AS IS" BASIS,
    WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
    See the License for the specific language governing permissions and
    limitations under the License.
*/

#include <assert.h>
#include <stddef.h>
#include <stdio.h>
#include <stdlib.h>
#include <string.h>
#include <unistd.h>

#include "umf.h"
#include "ze_memory_provider.h"

typedef struct ze_memory_provider_t {
    ze_context_handle_t context;
    ze_device_handle_t device;
    ze_device_mem_alloc_desc_t device_mem_alloc_desc;
} ze_memory_provider_t;

enum umf_result_t ze_memory_provider_initialize(void *params, void **provider) {
    ze_memory_provider_params_t *ze_params =
        (ze_memory_provider_params_t *)params;

    ze_memory_provider_t *ze_provider =
        (ze_memory_provider_t *)malloc(sizeof(ze_memory_provider_t));
    if (!ze_provider) {
        return UMF_RESULT_ERROR_OUT_OF_HOST_MEMORY;
    }

    ze_provider->context = ze_params->context;
    ze_provider->device = ze_params->device;
    ze_provider->device_mem_alloc_desc = ze_params->device_mem_alloc_desc;

    // TODO in this example we assume that ZE provider should offer only device memory
    assert(ze_provider->device_mem_alloc_desc.stype ==
           ZE_STRUCTURE_TYPE_DEVICE_MEM_ALLOC_DESC);

    // TODO add deep copy of mem_alloc_desc
    assert(ze_params->device_mem_alloc_desc.pNext == NULL);

    *provider = ze_provider;

    // TODO check results
    return UMF_RESULT_SUCCESS;
}

void ze_memory_provider_finalize(void *provider) {
    ze_memory_provider_t *ze_provider = (struct ze_memory_provider_t *)provider;

    // we don't alloc anything so just return
}

static enum umf_result_t ze_memory_provider_alloc(void *provider, size_t size,
                                                  size_t alignment,
                                                  void **resultPtr) {
    ze_memory_provider_t *ze_provider = (struct ze_memory_provider_t *)provider;

    zeMemAllocDevice(ze_provider->context, &ze_provider->device_mem_alloc_desc,
                     size, alignment, ze_provider->device, resultPtr);

    // TODO check results
    return UMF_RESULT_SUCCESS;
}

static enum umf_result_t ze_memory_provider_free(void *provider, void *ptr,
                                                 size_t bytes) {
    ze_memory_provider_t *ze_provider = (struct ze_memory_provider_t *)provider;

    zeMemFree(ze_provider->context, ptr);

    // TODO
    return UMF_RESULT_SUCCESS;
}

void ze_memory_provider_get_last_native_error(void *provider,
                                              const char **ppMessage,
                                              int32_t *pError) {
    //
}

static enum umf_result_t
ze_memory_provider_get_min_page_size(void *provider, void *ptr,
                                     size_t *pageSize) {
    *pageSize = 0; // TODO
    return UMF_RESULT_SUCCESS;
}

const char *ze_memory_provider_get_name(void *provider) { return "ZE"; }

struct umf_memory_provider_ops_t UMF_ZE_MEMORY_PROVIDER_OPS = {
    .version = UMF_VERSION_CURRENT,
    .initialize = ze_memory_provider_initialize,
    .finalize = ze_memory_provider_finalize,
    .alloc = ze_memory_provider_alloc,
    .free = ze_memory_provider_free,
    .get_last_native_error = ze_memory_provider_get_last_native_error,
    .get_min_page_size = ze_memory_provider_get_min_page_size,
    .get_name = ze_memory_provider_get_name,
};
