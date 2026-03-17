use std::env;

use oneapi_rs::unified_runtime::{
    result::{self, UnifiedRuntimeError},
    sys::{self, ur_adapter_handle_t, ur_device_type_t},
};

unsafe fn release_adapters(adapters: &[ur_adapter_handle_t]) {
    for &adapter in adapters {
        let _ = unsafe { result::adapter::release(adapter) };
    }
}

fn tear_down_loader() {
    let _ = result::loader::tear_down();
}

fn format_ur_error(error: UnifiedRuntimeError, call: &str) -> String {
    format!("{call} failed with error code {}", error.0 as u32)
}

fn run() -> Result<(), String> {
    if env::var_os("UR_ADAPTERS_SEARCH_PATH")
        .as_deref()
        .is_none_or(|value| value.is_empty())
    {
        eprintln!(
            "UR_ADAPTERS_SEARCH_PATH is not set. Point it at the Unified Runtime lib directory if adapter discovery fails."
        );
    }

    result::loader::init(0).map_err(|error| format_ur_error(error, "urLoaderInit"))?;

    let adapters = match result::adapter::get() {
        Ok(adapters) => adapters,
        Err(error) => {
            tear_down_loader();
            return Err(format_ur_error(error, "urAdapterGet"));
        }
    };

    if adapters.is_empty() {
        tear_down_loader();
        return Err("No Unified Runtime adapters were found.".to_string());
    }

    let mut platforms = Vec::new();
    for &adapter in &adapters {
        let adapter_platforms = match unsafe { result::platform::get(adapter) } {
            Ok(platforms) => platforms,
            Err(error) => {
                unsafe { release_adapters(&adapters) };
                tear_down_loader();
                return Err(format_ur_error(error, "urPlatformGet"));
            }
        };
        platforms.extend(adapter_platforms);
    }

    if platforms.is_empty() {
        unsafe { release_adapters(&adapters) };
        tear_down_loader();
        return Err("Adapters loaded, but no platforms were reported.".to_string());
    }

    let mut total_devices = 0usize;
    for &platform in &platforms {
        let platform_name = unsafe { result::platform::name(platform) }
            .unwrap_or_else(|_| "<unknown platform>".to_string());

        let devices =
            match unsafe { result::device::get(platform, ur_device_type_t::UR_DEVICE_TYPE_ALL) } {
                Ok(devices) => devices,
                Err(error) => {
                    unsafe { release_adapters(&adapters) };
                    tear_down_loader();
                    return Err(format_ur_error(error, "urDeviceGet"));
                }
            };

        println!("Platform: {platform_name} ({} device(s))", devices.len());

        for &device in &devices {
            let device_name = unsafe { result::device::name(device) }
                .unwrap_or_else(|_| "<unknown device>".to_string());
            println!("  - {device_name}");
            total_devices += 1;
        }
    }

    unsafe { release_adapters(&adapters) };
    tear_down_loader();

    if total_devices == 0 {
        return Err("No devices were reported by the loaded platforms.".to_string());
    }

    Ok(())
}

fn main() {
    let _ = sys::ur_device_type_t::UR_DEVICE_TYPE_ALL;

    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
