use std::{
    env,
    ffi::{CString, c_void},
    fs,
    path::{Path, PathBuf},
    process::Command,
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "../../../oneapi_helper.rs"]
mod oneapi_helper;

use oneapi_rs::unified_runtime::{
    result::UnifiedRuntimeError,
    safe::{Context, Queue},
    sys,
};

const DEFAULT_OCLOC_DEVICE: &str = "pvc";
const KERNEL_NAME: &str = "sin_kernel";

fn main() {
    oneapi_helper::configure_level_zero_runtime_environment();

    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let context = Context::new(0).map_err(|error| format!("Context::new failed: {error}"))?;
    let queue = context
        .new_queue()
        .map_err(|error| format!("Context::new_queue failed: {error}"))?;

    let spirv =
        compile_opencl_to_spirv().map_err(|error| format!("ocloc compile failed: {error}"))?;
    let program = Program::from_il(&context, &spirv)
        .map_err(|error| format!("Program::from_il failed: {error}"))?;
    program
        .build(&context)
        .map_err(|error| format!("Program::build failed: {error}"))?;
    let kernel = program
        .create_kernel(KERNEL_NAME)
        .map_err(|error| format!("Program::create_kernel failed: {error}"))?;

    let host_input = [0.0f32, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
    let device_input = queue
        .clone_htod(&host_input)
        .map_err(|error| format!("Queue::clone_htod failed: {error}"))?;
    let device_output = unsafe { queue.alloc::<f32>(host_input.len()) }
        .map_err(|error| format!("Queue::alloc failed: {error}"))?;

    unsafe {
        kernel
            .set_arg_pointer(0, device_output.as_mut_ptr().cast::<c_void>())
            .map_err(|error| format!("Kernel arg 0 failed: {error}"))?;
        kernel
            .set_arg_pointer(1, device_input.as_mut_ptr().cast::<c_void>())
            .map_err(|error| format!("Kernel arg 1 failed: {error}"))?;
        kernel
            .set_arg_value(2, &(host_input.len() as i32))
            .map_err(|error| format!("Kernel arg 2 failed: {error}"))?;
    }

    kernel
        .launch(&queue, host_input.len())
        .map_err(|error| format!("Kernel launch failed: {error}"))?;

    let mut host_output = vec![0.0f32; host_input.len()];
    copy_dtoh(&queue, &device_output, &mut host_output)
        .map_err(|error| format!("Device-to-host copy failed: {error}"))?;

    queue
        .synchronize()
        .map_err(|error| format!("Queue::synchronize failed: {error}"))?;

    for (input, output) in host_input.iter().zip(&host_output) {
        println!("sin({input:.3}) = {output:.6}");
    }

    let max_error = host_input
        .iter()
        .zip(&host_output)
        .map(|(input, output)| (input.sin() - output).abs())
        .fold(0.0f32, f32::max);

    println!("Max abs error: {max_error:.8}");

    Ok(())
}

fn compile_opencl_to_spirv() -> Result<Vec<u8>, String> {
    let ocloc_path = oneapi_helper::resolve_ocloc_path()?;

    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("sin.cl");
    let out_dir = make_temp_dir("oneapi-rs-launch-kernel")?;
    let output_base = "sin";
    let device = env::var("OCLOC_DEVICE").unwrap_or_else(|_| DEFAULT_OCLOC_DEVICE.to_string());
    let library_dir = ocloc_path
        .parent()
        .ok_or_else(|| format!("ocloc path has no parent: {}", ocloc_path.display()))?;

    let mut command = Command::new(&ocloc_path);
    command
        .arg("compile")
        .arg("-file")
        .arg(&source_path)
        .arg("-device")
        .arg(&device)
        .arg("-output")
        .arg(output_base)
        .arg("-out_dir")
        .arg(&out_dir)
        .env(
            "LD_LIBRARY_PATH",
            oneapi_helper::join_env_path("LD_LIBRARY_PATH", library_dir),
        );

    let output = command
        .output()
        .map_err(|error| format!("failed to spawn ocloc: {error}"))?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!(
            "command {:?} failed with status {}\nstdout:\n{}\nstderr:\n{}",
            command,
            output.status,
            stdout.trim(),
            stderr.trim()
        ));
    }

    let spirv_path = find_spirv_file(&out_dir)?;
    fs::read(&spirv_path)
        .map_err(|error| format!("failed to read {}: {error}", spirv_path.display()))
}

fn find_spirv_file(out_dir: &Path) -> Result<PathBuf, String> {
    let mut candidates = Vec::new();
    let entries = fs::read_dir(out_dir)
        .map_err(|error| format!("failed to read {}: {error}", out_dir.display()))?;

    for entry in entries {
        let entry =
            entry.map_err(|error| format!("failed to inspect {}: {error}", out_dir.display()))?;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "spv") {
            candidates.push(path);
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(format!(
            "ocloc did not emit a .spv file in {}",
            out_dir.display()
        )),
        _ => Err(format!(
            "ocloc emitted multiple .spv files in {}",
            out_dir.display()
        )),
    }
}

fn make_temp_dir(prefix: &str) -> Result<PathBuf, String> {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|error| format!("clock error: {error}"))?
        .as_nanos();
    let dir = env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&dir)
        .map_err(|error| format!("failed to create {}: {error}", dir.display()))?;
    Ok(dir)
}

fn copy_dtoh(
    queue: &Arc<Queue>,
    src: &oneapi_rs::unified_runtime::safe::UrDeviceSlice<f32>,
    dst: &mut [f32],
) -> Result<(), UnifiedRuntimeError> {
    let size = std::mem::size_of_val(dst);
    unsafe {
        sys::urEnqueueUSMMemcpy(
            queue.handle(),
            true,
            dst.as_mut_ptr().cast::<c_void>(),
            src.as_mut_ptr().cast::<c_void>(),
            size,
            0,
            std::ptr::null(),
            std::ptr::null_mut(),
        )
        .result()
    }
}

struct Program {
    handle: sys::ur_program_handle_t,
    device: sys::ur_device_handle_t,
}

impl Program {
    fn from_il(context: &Arc<Context>, il: &[u8]) -> Result<Self, UnifiedRuntimeError> {
        let mut handle = std::ptr::null_mut();
        unsafe {
            sys::urProgramCreateWithIL(
                context.handle(),
                il.as_ptr().cast::<c_void>(),
                il.len(),
                std::ptr::null(),
                &mut handle,
            )
            .result()?;
        }

        Ok(Self {
            handle,
            device: context.devices()[0],
        })
    }

    fn build(&self, context: &Arc<Context>) -> Result<(), String> {
        match unsafe { sys::urProgramBuild(context.handle(), self.handle, std::ptr::null()) }
            .result()
        {
            Ok(()) => Ok(()),
            Err(error) => Err(format!("{error}\nbuild log:\n{}", self.build_log())),
        }
    }

    fn create_kernel(&self, name: &str) -> Result<Kernel, UnifiedRuntimeError> {
        let mut handle = std::ptr::null_mut();
        let name = CString::new(name).expect("kernel name must not contain NUL");
        unsafe {
            sys::urKernelCreate(self.handle, name.as_ptr(), &mut handle).result()?;
        }
        Ok(Kernel { handle })
    }

    fn build_log(&self) -> String {
        get_program_build_info_string(
            self.handle,
            self.device,
            sys::ur_program_build_info_t::UR_PROGRAM_BUILD_INFO_LOG,
        )
        .unwrap_or_else(|error| format!("<unable to read build log: {error}>"))
    }
}

impl Drop for Program {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            let _ = unsafe { sys::urProgramRelease(self.handle).result() };
            self.handle = std::ptr::null_mut();
        }
    }
}

struct Kernel {
    handle: sys::ur_kernel_handle_t,
}

impl Kernel {
    unsafe fn set_arg_pointer(
        &self,
        index: u32,
        pointer: *const c_void,
    ) -> Result<(), UnifiedRuntimeError> {
        unsafe {
            sys::urKernelSetArgPointer(self.handle, index, std::ptr::null(), pointer).result()
        }
    }

    unsafe fn set_arg_value<T>(&self, index: u32, value: &T) -> Result<(), UnifiedRuntimeError> {
        unsafe {
            sys::urKernelSetArgValue(
                self.handle,
                index,
                std::mem::size_of::<T>(),
                std::ptr::null(),
                (value as *const T).cast::<c_void>(),
            )
            .result()
        }
    }

    fn launch(
        &self,
        queue: &Arc<Queue>,
        global_work_items: usize,
    ) -> Result<(), UnifiedRuntimeError> {
        let global = [global_work_items];
        unsafe {
            sys::urEnqueueKernelLaunch(
                queue.handle(),
                self.handle,
                1,
                std::ptr::null(),
                global.as_ptr(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                std::ptr::null(),
                std::ptr::null_mut(),
            )
            .result()
        }
    }
}

impl Drop for Kernel {
    fn drop(&mut self) {
        if !self.handle.is_null() {
            let _ = unsafe { sys::urKernelRelease(self.handle).result() };
            self.handle = std::ptr::null_mut();
        }
    }
}

fn get_program_build_info_string(
    program: sys::ur_program_handle_t,
    device: sys::ur_device_handle_t,
    info: sys::ur_program_build_info_t,
) -> Result<String, UnifiedRuntimeError> {
    let mut size = 0usize;
    unsafe {
        sys::urProgramGetBuildInfo(program, device, info, 0, std::ptr::null_mut(), &mut size)
            .result()?;
    }
    if size == 0 {
        return Ok(String::new());
    }

    let mut bytes = vec![0u8; size];
    unsafe {
        sys::urProgramGetBuildInfo(
            program,
            device,
            info,
            bytes.len(),
            bytes.as_mut_ptr().cast::<c_void>(),
            std::ptr::null_mut(),
        )
        .result()?;
    }

    let string = String::from_utf8_lossy(&bytes);
    Ok(string.trim_end_matches('\0').to_string())
}
