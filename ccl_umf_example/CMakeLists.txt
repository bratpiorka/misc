cmake_minimum_required (VERSION 3.4)
project(umf_example CXX)

add_executable(umf_example 
	umf_example.cpp)

target_include_directories(umf_example PRIVATE 
	${CMAKE_CURRENT_SOURCE_DIR} 
	${CMAKE_CURRENT_SOURCE_DIR}/umf/umf/include)

target_link_libraries(umf_example PRIVATE unified_memory_framework ze_loader)

add_subdirectory(umf)
