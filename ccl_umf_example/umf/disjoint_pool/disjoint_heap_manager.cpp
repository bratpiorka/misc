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

#include <dlfcn.h>
#include <stdbool.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>

#include <pthread.h>

#include <umf/memory_pool_ops.h>
#include <umf/memory_provider.h>

#include "disjoint_pool/disjoint_pool.h"
#include "disjoint_pool/disjoint_pool.hpp"

struct disjoint_memory_pool {
    umf_memory_provider_handle_t mem_provider;
    usm::DisjointPool *disjoint_pool;
};

enum umf_result_t
disjoint_pool_initialize(umf_memory_provider_handle_t *providers,
                         size_t numProviders, void *params, void **pool) {
    // NOTE: ignore params and use hardcoded vals
    usm::DisjointPoolConfig config{};
    config.SlabMinSize = 4096;
    config.MaxPoolableSize = 4096;
    config.Capacity = 4;
    config.MinBucketSize = 64;

    struct disjoint_memory_pool *pool_data =
        (struct disjoint_memory_pool *)malloc(
            sizeof(struct disjoint_memory_pool));
    if (!pool_data) {
        fprintf(stderr, "cannot allocate memory for metadata\n");
        abort();
    }

    pool_data->mem_provider = providers[0];
    pool_data->disjoint_pool = new usm::DisjointPool();
    pool_data->disjoint_pool->initialize(providers, numProviders, config);

    *pool = (void *)pool_data;
    return UMF_RESULT_SUCCESS;
}

void disjoint_pool_finalize(void *pool) {
    struct disjoint_memory_pool *pool_data =
        (struct disjoint_memory_pool *)pool;
    delete pool_data->disjoint_pool;
    free((struct disjoint_memory_pool *)pool);
    pool = NULL;
}

void *disjoint_malloc(void *pool, size_t size) {
    struct disjoint_memory_pool *pool_data =
        (struct disjoint_memory_pool *)pool;
    return pool_data->disjoint_pool->malloc(size);
}

void *disjoint_calloc(void *pool, size_t num, size_t size) {
    return 0; // TODO
}

void *disjoint_realloc(void *pool, void *ptr, size_t size) {
    return 0; // TODO
}

void *disjoint_aligned_malloc(void *pool, size_t size, size_t alignment) {
    struct disjoint_memory_pool *pool_data =
        (struct disjoint_memory_pool *)pool;
    return pool_data->disjoint_pool->aligned_malloc(size, alignment);
}

enum umf_result_t disjoint_free(void *pool, void *ptr) {
    struct disjoint_memory_pool *pool_data =
        (struct disjoint_memory_pool *)pool;
    pool_data->disjoint_pool->free(ptr);
    return UMF_RESULT_SUCCESS;
}

enum umf_result_t disjoint_get_last_allocation_error(void *pool) {
    struct disjoint_memory_pool *pool_data =
        (struct disjoint_memory_pool *)pool;
    /* TODO */
    return UMF_RESULT_SUCCESS;
}

struct umf_memory_pool_ops_t DISJOINT_POOL_OPS = {
    .version = UMF_VERSION_CURRENT,
    .initialize = disjoint_pool_initialize,
    .finalize = disjoint_pool_finalize,
    .malloc = disjoint_malloc,
    .calloc = disjoint_calloc,
    .realloc = disjoint_realloc,
    .aligned_malloc = disjoint_aligned_malloc,
    .malloc_usable_size = NULL,
    .free = disjoint_free,
    .get_last_allocation_error = disjoint_get_last_allocation_error,
};
