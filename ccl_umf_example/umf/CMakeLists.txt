
project(unified_memory_framework CXX)

add_library(unified_memory_framework STATIC
    umf/src/memory_pool.cpp
    umf/src/memory_pool_default.cpp
    umf/src/memory_provider.cpp
    umf/src/memory_tracker.cpp
    umf/src/memory_provider_get_last_failed.cpp
    disjoint_pool/disjoint_heap_manager.cpp
    disjoint_pool/disjoint_pool.cpp
    memory_providers/os_memory_provider.cpp
    memory_providers/ze_memory_provider.cpp)

target_include_directories(unified_memory_framework PUBLIC 
    ${CMAKE_SOURCE_DIR}
    ${CMAKE_SOURCE_DIR}/umf
    ${CMAKE_SOURCE_DIR}/umf/umf/include)
