mod config;

use config::Config;
use std::env;
use std::path::PathBuf;

const VERSION: &str = env!("CARGO_PKG_VERSION");

fn main() {
    if let Err(error) = run() {
        eprintln!("alpinium: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let mut explicit_config: Option<PathBuf> = None;
    let mut check_config = false;
    let mut print_config = false;

    while let Some(argument) = args.next() {
        match argument.to_str() {
            Some("--version") | Some("-V") => {
                println!("Alpinium {VERSION}");
                return Ok(());
            }
            Some("--help") | Some("-h") => {
                print_help();
                return Ok(());
            }
            Some("--check-config") => check_config = true,
            Some("--print-config") => print_config = true,
            Some("--config") => {
                let path = args
                    .next()
                    .ok_or_else(|| "--config requires a file path".to_string())?;
                explicit_config = Some(PathBuf::from(path));
            }
            Some(other) => return Err(format!("unknown argument '{other}'; use --help")),
            None => return Err("arguments must be valid UTF-8".to_string()),
        }
    }

    let (config, source) = Config::load(explicit_config.as_deref())?;
    let source_description = source
        .as_ref()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "built-in defaults (no config file found)".to_string());

    if check_config {
        println!("Configuration valid.");
        println!("Source: {source_description}");
        println!("Mode: {}", config.mode);
        return Ok(());
    }

    if print_config {
        println!("# Loaded from: {source_description}");
        println!("mode {}", config.mode);
        println!(
            "animations {}",
            if config.animations { "enabled" } else { "disabled" }
        );
        println!("animation_duration {}ms", config.animation_duration_ms);
        println!("gaps inner {}", config.inner_gap);
        println!("gaps outer {}", config.outer_gap);
        println!("corner_radius {}", config.corner_radius);
        println!("shadows {}", if config.shadows { "enabled" } else { "disabled" });
        return Ok(());
    }

    println!("Alpinium {VERSION} — compositor development build");
    println!("Loaded configuration: {source_description}");
    println!("Selected layout mode: {}", config.mode);
    println!("The Wayland compositor backend is not implemented yet.");
    println!("Use --check-config to validate a config or --help for options.");
    Ok(())
}

fn print_help() {
    println!("Alpinium {VERSION}");
    println!("A configurable Wayland compositor for KhaymOS.\n");
    println!("Usage: alpinium [OPTIONS]\n");
    println!("Options:");
    println!("  --config PATH     Load a specific configuration file");
    println!("  --check-config    Parse and validate the selected configuration");
    println!("  --print-config    Print the effective configuration");
    println!("  --version         Print version information");
    println!("  -h, --help        Print this help message");
}
