use std::{
    env,
    error::Error,
    ffi::c_void,
    fs,
    io,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

#[path = "../../../oneapi_helper.rs"]
mod oneapi_helper;

use oneapi_rs::unified_runtime::safe::{DevicePtr, DevicePtrMut, UrContext};

const DEFAULT_OCLOC_DEVICE: &str = "pvc";
const KERNEL_NAME: &str = "sin_kernel";

type ExampleResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    oneapi_helper::configure_level_zero_runtime_environment();

    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> ExampleResult<()> {
    let context = UrContext::new(0)?;
    let queue = context.new_queue()?;

    let spirv = compile_opencl_to_spirv()?;
    let program = context.load_program(&spirv)?;
    if let Err(error) = program.build() {
        let build_log = program
            .build_log()
            .unwrap_or_else(|build_error| format!("<unable to read build log: {build_error}>"));
        return Err(io::Error::other(format!("{error}\nbuild log:\n{build_log}")).into());
    }
    let kernel = program.create_kernel(KERNEL_NAME)?;

    let host_input = [0.0f32, 0.5, 1.0, 1.5, 2.0, 2.5, 3.0, 3.5];
    let device_input = queue.clone_htod(&host_input)?;
    let mut device_output = unsafe { queue.alloc::<f32>(host_input.len()) }?;

    unsafe {
        kernel.set_arg_pointer(0, device_output.device_ptr_mut().cast::<c_void>())?;
        kernel.set_arg_pointer(1, device_input.device_ptr().cast::<c_void>())?;
        kernel.set_arg_value(2, &(host_input.len() as i32))?;
    }

    kernel.launch(&queue, host_input.len())?;

    let mut host_output = vec![0.0f32; host_input.len()];
    queue.memcpy_dtoh(&device_output, &mut host_output)?;

    queue.synchronize()?;

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

fn compile_opencl_to_spirv() -> ExampleResult<Vec<u8>> {
    let ocloc_path = match oneapi_helper::resolve_ocloc_path() {
        Ok(path) => path,
        Err(error) => return Err(io::Error::other(error).into()),
    };

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
        .output()?;
    if !output.status.success() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(io::Error::other(format!(
            "command {:?} failed with status {}\nstdout:\n{}\nstderr:\n{}",
            command,
            output.status,
            stdout.trim(),
            stderr.trim()
        ))
        .into());
    }

    let spirv_path = find_spirv_file(&out_dir)?;
    Ok(fs::read(&spirv_path)?)
}

fn find_spirv_file(out_dir: &Path) -> ExampleResult<PathBuf> {
    let mut candidates = Vec::new();
    let entries = fs::read_dir(out_dir)?;

    for entry in entries {
        let entry = entry?;
        let path = entry.path();
        if path.extension().is_some_and(|extension| extension == "spv") {
            candidates.push(path);
        }
    }

    match candidates.len() {
        1 => Ok(candidates.remove(0)),
        0 => Err(io::Error::new(
            io::ErrorKind::NotFound,
            format!("ocloc did not emit a .spv file in {}", out_dir.display()),
        )
        .into()),
        _ => Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("ocloc emitted multiple .spv files in {}", out_dir.display()),
        )
        .into()),
    }
}

fn make_temp_dir(prefix: &str) -> ExampleResult<PathBuf> {
    let unique = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos();
    let dir = env::temp_dir().join(format!("{prefix}-{unique}"));
    fs::create_dir_all(&dir)?;
    Ok(dir)
}
