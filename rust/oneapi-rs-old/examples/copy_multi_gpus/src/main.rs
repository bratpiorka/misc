use std::error::Error;

#[path = "../../../oneapi_helper.rs"]
mod oneapi_helper;

use oneapi_rs::unified_runtime::safe::UrContext;

fn main() {
    oneapi_helper::configure_runtime_environment();

    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let size = 10;
    let expected: Vec<f64> = (1..=size).map(|value| value as f64).collect();

    // TODO check > 1
    UrContext::device_count()?;

    let ctx1 = UrContext::new(0)?;
    let queue1 = ctx1.new_queue()?;
    let a = queue1.clone_htod(&expected)?;

    let ctx2 = UrContext::new(1)?;
    let queue2 = ctx2.new_queue()?;
    let b = queue2.alloc_zeros::<f64>(size)?;

    let a_host = queue1.clone_dtoh(&a)?;
    queue2.memcpy_htod(&a_host, &b)?;

    let b_host = queue2.clone_dtoh(&b)?;

    assert_eq!(a_host, expected);
    assert_eq!(a_host, b_host);

    Ok(())
}