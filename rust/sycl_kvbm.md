SYCL Wrappers Relative to cudarc KVBM List
oneapi-rs: /home/rrudnick/rust/rust/oneapi-rs

1. High-Level Wrappers Mapping

| cudarc::driver item | Kind | SYCL counterpart in oneapi-rs | SYCL API used | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| CudaContext | struct | SyclContext | `sycl::context` | Present | High-level owned context wrapper. |
| CudaStream | struct | SyclQueue | `sycl::queue` | Present | Queue is the closest SYCL execution-stream counterpart. |
| CudaSlice | struct | SyclBuffer | USM pointer + allocation kind | Present | Generic allocation wrapper for device/shared/host USM. |
| CudaEvent | struct | SyclEvent | `sycl::event` | Partial | Event wrapper exists and is currently used for async memcpy completion. |
| CudaFunction | struct | SyclKernel | `sycl::kernel` | Present | Kernel wrapper backed by runtime-compiled SYCL kernel objects. |
| CudaModule | struct | SyclProgram | `sycl::kernel_bundle<executable>` | Present | Program-like wrapper built from source via the experimental kernel compiler extension. |
| DriverError | error type | SyclError | `sycl::exception::what()` via shim | Present | Error type backed by shim result code plus message. |
| DevicePtr | trait/type | DevicePtr | `*const T` view over USM-backed storage | Partial | Present as a safe-layer trait and currently implemented for `SyclBuffer<T>`. |
| DevicePtrMut | trait/type | DevicePtrMut | `*mut T` view over USM-backed storage | Partial | Present as a safe-layer trait and currently implemented for `SyclBuffer<T>`. |
| DeviceRepr | trait | DeviceCopy | typed memcpy-compatible values | Partial | Covers copy-safe POD-style values for transfers; kernel arguments are handled separately through `SyclKernelArg`. |
| ValidAsZeroBits | trait | ValidAsZeroBits | zero-initializable typed USM allocation contract | Present | Used by `SyclQueue::alloc_zeros()` to guarantee all-zero initialization is valid for the element type. |
| LaunchConfig | struct | None | `SyclKernel::launch_1d(...)` with explicit global/local sizes | Partial | There is a direct 1D launch entrypoint, but no reusable launch-config struct. |
| PushKernelArg | trait | SyclKernelArg | `handler::set_arg` + `raw_kernel_arg` | Partial | Typed kernel-argument builder exists as a value type, and now works naturally with `DevicePtr` / `DevicePtrMut`. |
| CudaContext::new() | method | SyclContext::new() | `sycl::context(device)` | Present | Builds a context from a selected device. |
| CudaContext::device_count() | method | SyclContext::device_count() | `sycl::device::get_devices(sycl::info::device_type::all)` | Present | Returns the number of visible SYCL devices discovered across the runtime. |
| CudaContext::default_stream() | method | SyclQueue::new_default() | `sycl::device(sycl::default_selector_v)`, `sycl::context`, `sycl::queue` | Partial | Creates default device+context+queue together, not a stream off an existing context. |
| CudaContext::new_stream() | method | SyclQueue::new() | `sycl::queue(context, device)` | Partial | Creates a queue from an existing context. |
| CudaContext::ordinal() | method | None | — | Missing | No device ordinal tracking. |
| CudaContext::attribute() | method | None | — | Missing | No device/context info queries yet. |
| CudaContext::cu_device() | method | SyclContext::device() | wrapped `sycl::device` | Partial | Returns wrapped SYCL device, not a raw CUDA handle. |
| CudaContext::cu_ctx() | method | SyclContext::handle() | shim-owned `sycl::context` handle | Partial | Returns shim handle, not native backend handle. |
| CudaContext::bind_to_thread() | method | None | — | Missing | No thread-binding concept exposed. |
| CudaContext::load_module() | method | SyclContext::load_program_from_source() | `create_kernel_bundle_from_source`, `build` | Partial | Loads from in-memory SYCL source rather than binary/PTX/module blobs. |
| CudaModule::load_function() | method | SyclProgram::create_kernel() | `kernel_bundle::ext_oneapi_get_kernel` | Present | Looks up a runtime-compiled free-function kernel by name. |
| CudaStream::cu_stream() | method | SyclQueue::handle() | shim-owned `sycl::queue` handle | Partial | Returns shim queue handle, not backend-native stream/queue. |
| CudaStream::context() | method | SyclQueue::context() | wrapped `sycl::context` | Present | Accessor for owning context. |
| CudaStream::alloc() | method | SyclQueue::alloc_device() | `sycl::malloc_device` / `sycl::aligned_alloc_device` | Present | Unsafe typed allocation helper. |
| CudaStream::alloc_zeros() | method | SyclQueue::alloc_zeros() | `sycl::malloc_device` + `sycl::queue::memset` | Present | Device allocation followed by explicit zero-fill for `ValidAsZeroBits` element types. |
| CudaStream::clone_htod() | method | None | use `alloc_device()` + `sycl::memcpy(...)` | Missing | Removed in favor of explicit allocation plus neutral `memcpy`. |
| CudaStream::clone_dtoh() | method | SyclQueue::clone_dtoh() | `sycl::queue::memcpy` | Present | Copies from buffer into a new host vector convenience helper. |
| CudaStream::synchronize() | method | SyclQueue::synchronize() | `sycl::queue::wait` | Present | Queue wait helper. |
| CudaStream::record_event() | method | None | — | Missing | Event objects exist, but there is no standalone record-event API yet. |
| CudaStream::launch_builder() | method | None | `SyclKernel::launch_1d(...)` only | Partial | Direct launch exists, but no builder-style enqueue API. |
| CudaSlice::device_ptr() | method | DevicePtr::device_ptr() | underlying USM pointer | Partial | Trait is present and implemented for `SyclBuffer<T>`. |
| CudaSlice::device_ptr_mut() | method | DevicePtrMut::device_ptr_mut() | underlying USM pointer | Partial | Trait is present and implemented for `SyclBuffer<T>`. |

2. Safe FFI Wrappers Mapping

| cudarc::driver::result item | Kind | SYCL counterpart in oneapi-rs | SYCL API used | Status | Notes |
| --- | --- | --- | --- | --- | --- |
| malloc_sync | function | SyclQueue::alloc_device(), alloc_shared(), alloc_host() | `sycl::malloc_device`, `sycl::malloc_shared`, `sycl::malloc_host`, aligned variants | Partial | Present as high-level allocation helpers rather than freestanding result-layer functions. |
| free_sync | function | SyclBuffer::drop() | `sycl::free` | Partial | Free is automatic through RAII; no explicit safe wrapper function. |
| malloc_host | function | SyclQueue::alloc_host() | `sycl::malloc_host` / `sycl::aligned_alloc_host` | Present | Host USM allocation helper. |
| free_host | function | SyclBuffer::drop() | `sycl::free` | Partial | Host USM also frees through RAII. |
| memcpy_htod_async | function | `sycl::memcpy(...)` | `sycl::queue::memcpy` returning `sycl::event` | Partial | Direction is neutral in the SYCL API; `memcpy(...)` is now the async default and returns `SyclEvent`. |
| memcpy_dtoh_async | function | `sycl::memcpy(...)` | `sycl::queue::memcpy` returning `sycl::event` | Partial | Same neutral async API as above. |
| memset_d8_async | function | None | — | Missing | No memset/fill wrapper yet. |
| launch_kernel | function/pattern | SyclKernel::launch_1d() | `handler::set_arg`, `raw_kernel_arg`, `parallel_for(nd_range, kernel)` | Partial | Present for 1D launches only and exposed through the high-level wrapper rather than the result layer. |
| module/program build | function/pattern | SyclContext::load_program_from_source(), SyclQueue::load_program_from_source() | `create_kernel_bundle_from_source`, `build`, `save_log` | Partial | Runtime source compilation is supported; binary/module ingestion is not. |

3. Present SYCL Surface Summary
- High-level wrappers currently present: SyclDevice, SyclContext, SyclQueue, SyclEvent, SyclBuffer, SyclProgram, SyclKernel, SyclKernelArg, DevicePtr, DevicePtrMut, ValidAsZeroBits, SyclError.
- Copy API is intentionally neutral and centralized as async `sycl::memcpy(&queue, src, dst)` plus blocking `sycl::memcpy_sync(&queue, src, dst)` rather than direction-specific helpers.
- Buffer allocation supports device, shared, and host USM via `alloc_device`, `alloc_shared`, and `alloc_host`, plus zero-initialized device allocation through `alloc_zeros`.
- `clone_htod` has been removed in favor of explicit `alloc_device(...)` plus `sycl::memcpy(...)`; `clone_dtoh` remains as a convenience helper.
- Runtime kernel compilation and execution are now supported through `load_program_from_source`, `create_kernel`, `SyclKernelArg`, `DevicePtr` / `DevicePtrMut`, and `launch_1d`.
- Device discovery now includes `SyclContext::device_count()` and ordinal-based queue/context creation helpers for multi-device workflows.
- Example execution is currently done through the crate-local `run_examples.sh`, which sources oneAPI `setvars.sh` before running the SYCL examples, including the multi-GPU path.

4. What Is Missing
- Event model: `SyclEvent` and async memcpy completion now exist, but there is still no event query/status API or standalone record-event helper.
- Richer kernel execution: no binary/module loading, no multi-dimensional launch helpers, no launch-config struct, and no builder-style enqueue API.
- Device inspection: no device names, ordinal metadata, or attribute queries yet beyond the visible-device count helper.
- Pointer ergonomics: `DevicePtr` / `DevicePtrMut` now exist for `SyclBuffer<T>`, but there are still no subview/slice abstractions or broader pointer-wrapper ecosystem comparable to cudarc.
- Memory utilities: no general fill helper beyond `alloc_zeros`, and no explicit free functions outside RAII drop.
- Native interop: current `handle()` accessors expose shim-owned opaque handles, not backend-native handles.

5. Recommended Next Additions
1	SyclEvent plus async memcpy support
2	memset/fill and alloc_zeros helpers
3	device enumeration and attribute queries
4	launch configuration and multi-dimensional kernel helpers
5	richer typed buffer/subview ergonomics beyond `SyclBuffer<T>`