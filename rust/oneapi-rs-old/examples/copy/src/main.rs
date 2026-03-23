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

    let a = queue.alloc_zeros::<f64>(10)?;
    let b = queue.alloc_zeros::<f64>(10)?;

    queue.memcpy_dtod(&a, &b)?;

    queue.memcpy_htod(&vec![2.0; b.len()], &b)?;
    queue.memcpy_htod(&[3.0; 10], &b)?;

    let mut a_host = queue.clone_dtoh(&a)?;
    assert_eq!(a_host, [0.0; 10]);

    let b_host = queue.clone_dtoh(&b)?;
    assert_eq!(b_host, [3.0; 10]);

    queue.memcpy_dtoh(&b, &mut a_host)?;
    assert_eq!(a_host, b_host);

    Ok(())
}
