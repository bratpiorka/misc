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

#include <level_zero/ze_api.h>

typedef struct ze_memory_provider_params_t {
    ze_context_handle_t context;
    ze_device_handle_t device;
    ze_device_mem_alloc_desc_t device_mem_alloc_desc;
} ze_memory_provider_params_t;

extern struct umf_memory_provider_ops_t UMF_ZE_MEMORY_PROVIDER_OPS;

#ifdef __cplusplus
}
#endif
