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

use oneapi_rs::unified_runtime::safe::{DevicePtrMut, DeviceRepr, UrContext, UrProgram};

const DEFAULT_OCLOC_DEVICE: &str = "pvc";
const KERNEL_NAME: &str = "sum_struct_fields";

type ExampleResult<T> = Result<T, Box<dyn Error>>;

#[repr(C)]
#[derive(Clone, Copy, Debug)]
struct ExampleData {
    a: [u8; 8],
    b: [f32; 8],
}

unsafe impl DeviceRepr for ExampleData {}

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
    build_program(&program)?;
    let kernel = program.create_kernel(KERNEL_NAME)?;

    let input = ExampleData {
        a: [0, 1, 2, 3, 4, 5, 6, 7],
        b: [0.1, 0.2, 0.3, 0.4, 0.5, 0.6, 0.7, 0.8],
    };

    let mut device_output = queue.alloc_zeros::<f32>(1)?;

    unsafe {
        kernel.set_arg_value(0, &input)?;
        kernel.set_arg_pointer(1, device_output.device_ptr_mut().cast::<c_void>())?;
    }

    kernel.launch(&queue, 1)?;

    let mut host_output = [0.0f32; 1];
    queue.memcpy_dtoh(&device_output, &mut host_output)?;
    queue.synchronize()?;

    let expected = input.a.iter().map(|&value| value as f32).sum::<f32>()
        + input.b.iter().copied().sum::<f32>();

    assert!((host_output[0] - expected).abs() < 1.0e-5);

    println!("struct sum = {:.6}", host_output[0]);

    Ok(())
}

fn build_program(program: &UrProgram) -> ExampleResult<()> {
    if let Err(error) = program.build() {
        let build_log = program
            .build_log()
            .unwrap_or_else(|build_error| format!("<unable to read build log: {build_error}>"));
        return Err(io::Error::other(format!("{error}\nbuild log:\n{build_log}")).into());
    }

    Ok(())
}

fn compile_opencl_to_spirv() -> ExampleResult<Vec<u8>> {
    let ocloc_path = match oneapi_helper::resolve_ocloc_path() {
        Ok(path) => path,
        Err(error) => return Err(io::Error::other(error).into()),
    };

    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/kernel.cl");
    let out_dir = make_temp_dir("oneapi-rs-device-repr-struct")?;
    let output_base = "device_repr_struct";
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

    let output = command.output()?;
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