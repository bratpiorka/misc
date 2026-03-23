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

use oneapi_rs::safe::UrProgram;
use oneapi_rs::unified_runtime::safe::UrContext;

const DEFAULT_OCLOC_DEVICE: &str = "pvc";
const INIT_KERNEL_NAME: &str = "init_values";
const ADD_ONE_KERNEL_NAME: &str = "add_one";

type ExampleResult<T> = Result<T, Box<dyn Error>>;

fn main() {
    oneapi_helper::configure_level_zero_runtime_environment();

    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> ExampleResult<()> {
    let size = 1 << 20;
    let element_count = u32::try_from(size)
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "size does not fit in u32"))?;

    if UrContext::device_count()? < 2 {
        return Err(io::Error::new(
            io::ErrorKind::NotFound,
            "copy_multi_gpus_async requires at least 2 devices",
        )
        .into());
    }

    let ctx1 = UrContext::new(0)?;
    let queue1 = ctx1.new_queue()?;
    let ctx2 = UrContext::new(1)?;
    let queue2 = ctx2.new_queue()?;

    let spirv = compile_opencl_to_spirv()?;

    let program1 = ctx1.load_program(&spirv)?;
    build_program(&program1)?;
    let init_kernel = program1.create_kernel(INIT_KERNEL_NAME)?;

    let program2 = ctx2.load_program(&spirv)?;
    build_program(&program2)?;
    let add_one_kernel = program2.create_kernel(ADD_ONE_KERNEL_NAME)?;

    let src = queue1.alloc_zeros::<u32>(size)?;
    unsafe {
        init_kernel.set_arg_pointer(0, src.as_mut_ptr().cast::<c_void>())?;
        init_kernel.set_arg_value(1, &element_count)?;
    }
    init_kernel.launch(&queue1, size)?;

    let init_done = queue1.record_event()?;
    let mut staged = vec![0u32; size];
    let copy_to_host = unsafe { queue1.memcpy_dtoh_async(&src, &mut staged, &[&init_done])? };
    copy_to_host.wait()?;

    let dst = queue2.alloc_zeros::<u32>(size)?;
    let copy_to_device = unsafe { queue2.memcpy_htod_async(&staged, &dst, &[])? };

    unsafe {
        add_one_kernel.set_arg_pointer(0, dst.as_mut_ptr().cast::<c_void>())?;
        add_one_kernel.set_arg_value(1, &element_count)?;
    }
    let add_one_done = add_one_kernel.launch_async(&queue2, size, &[&copy_to_device])?;

    let mut result = vec![0u32; size];
    let copy_back = unsafe { queue2.memcpy_dtoh_async(&dst, &mut result, &[&add_one_done])? };

    // assert!(!copy_back.query()?); // could not be true
    copy_back.wait()?;
    assert!(copy_back.query()?);

    let expected_source: Vec<u32> = (0..element_count).collect();
    let expected_result: Vec<u32> = (1..=element_count).collect();

    assert_eq!(staged, expected_source);
    assert_eq!(result, expected_result);

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

    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src/kernels.cl");
    let out_dir = make_temp_dir("oneapi-rs-copy-multi-gpus-async")?;
    let output_base = "copy_multi_gpus_async";
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