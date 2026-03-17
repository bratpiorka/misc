use std::env;

use oneapi_rs::unified_runtime::safe::Context;

fn main() {
    if env::var_os("UR_ADAPTERS_SEARCH_PATH")
        .as_deref()
        .is_none_or(|value| value.is_empty())
    {
        eprintln!(
            "UR_ADAPTERS_SEARCH_PATH is not set. Point it at the Unified Runtime lib directory if adapter discovery fails."
        );
    }

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

    let host = [1.0f32, 2.0, 3.0, 4.0];
    let copied = queue
        .clone_htod(&host)
        .map_err(|error| format!("Queue::clone_htod failed: {error}"))?;

    queue
        .synchronize()
        .map_err(|error| format!("Queue::synchronize failed: {error}"))?;

    println!("Copied {} f32 values from host to device.", copied.len());

    Ok(())
}
