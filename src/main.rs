use anyhow::Result;
use clap::{Arg, Command};
use serde_yaml;
use std::process::exit;

mod adjuster;
mod config;
mod logger;
mod matcher;
mod monitor;

use crate::logger::init_logger;

/// Main entry point for the process monitoring daemon.
/// It parses the command-line arguments, initializes logging, and starts the event loop.
///
/// # Arguments
///
/// * None
///
/// # Returns
///
/// * `Ok(())` on success.
/// * `Err(anyhow::Error)` if an error occurs.
#[tokio::main]
async fn main() -> Result<(), anyhow::Error> {
    let matches = Command::new("reniced")
        .version("0.1.0")
        .author("Julian Kahlert <https://juliankahlert.github.io/reniced>")
        .about("A daemon to monitor and adjust nice values of processes based on configuration")
        .arg(
            Arg::new("show-config")
                .long("show-config")
                .help("Show the merged configuration and exit")
                .action(clap::ArgAction::SetTrue),
        )
        .arg(
            Arg::new("log-level")
                .long("log-level")
                .help("Set the logging level (debug, info, etc.)")
                .value_parser(["debug", "info", "warn", "error", "trace"])
                .action(clap::ArgAction::Set),
        )
        .get_matches();

    let log_level = matches.get_one::<String>("log-level").map(|x| x.as_str());
    init_logger(log_level);

    if matches.get_flag("show-config") {
        match show_merged_config() {
            Ok(_) => exit(0),
            Err(err) => {
                eprintln!("Error showing config: {}", err);
                exit(1);
            }
        }
    }

    info!("Starting process monitoring...");
    monitor::event_loop().await?; // Call the event loop from the monitor module

    Ok(())
}

/// Displays the merged global and local configurations in YAML format.
///
/// # Returns
///
/// * `Ok(())` if the configuration is successfully printed.
/// * `Err(anyhow::Error)` if there's an error during the process.
fn show_merged_config() -> anyhow::Result<()> {
    let merged_config = config::Config::load_all().unwrap_or_default();

    let yaml_output = serde_yaml::to_string(&merged_config)?;
    println!("{}", yaml_output);

    Ok(())
}
