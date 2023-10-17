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

#pragma once
#ifdef __cplusplus
extern "C" {
#endif

#include "umf/memory_pool.h"
#include "umf/memory_provider.h"

typedef struct disjoint_memory_pool_params_t {
    size_t max_size;
} disjoint_memory_pool_params_t;

extern struct umf_memory_pool_ops_t DISJOINT_POOL_OPS;

#ifdef __cplusplus
}
#endif
