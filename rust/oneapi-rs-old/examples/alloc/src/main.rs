#[path = "../../../oneapi_helper.rs"]
mod oneapi_helper;

use oneapi_rs::unified_runtime::{result::UrError, safe::UrContext};

fn main() {
    oneapi_helper::configure_runtime_environment();

    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), UrError> {
    let context = UrContext::new(0)?;
    let queue = context.new_queue()?;

    let host = [1.0f32, 2.0, 3.0, 4.0];
    let copied = queue.clone_htod(&host)?;

    queue.synchronize()?;

    println!("Copied {} f32 values from host to device.", copied.len());

    Ok(())
}
