use anyhow::Result;
use nix::unistd::{Uid, User};
use std::collections::HashSet;
use std::fs;
use std::io::Read;
use std::time::Duration;

use crate::{adjuster, config, matcher};
use crate::{debug, error, warn};

/// The main event loop of the process monitoring daemon.
/// This function continuously checks running processes, compares them with the previous state,
/// and adjusts the "nice" values of processes based on the configuration.
///
/// # Arguments
///
/// * `None`
///
/// # Returns
///
/// * `Ok(())` when the event loop completes successfully.
/// * `Err(anyhow::Error)` if an error occurs during execution.
pub async fn event_loop() -> Result<()> {
    let mut previous_pids = HashSet::new();
    let config = config::Config::load_all().unwrap_or_default();
    let matcher = matcher::ProcessMatcher::new(&config);
    let adjuster = adjuster::Adjuster::new(&config);

    loop {
        let current_pids = match get_running_processes() {
            Ok(pids) => pids,
            Err(e) => {
                error!("Error fetching processes: {}", e);
                continue;
            }
        };

        let added = current_pids.difference(&previous_pids).collect::<Vec<_>>();

        for pid in added {
            if let Some(command) = get_command_for_pid(pid) {
                if let Some(owner) = get_owner_for_pid(pid) {
                    if let Some(process_config) = matcher.match_command(&command, &owner) {
                        debug!(
                            "Process {} with command '{}' and owner '{}' matches config",
                            pid, command, owner
                        );
                        if let Ok(pid_int) = pid.parse::<i32>() {
                            adjuster.check_and_adjust_nice_value(pid_int, process_config);
                        }
                    }
                } else {
                    warn!("Failed to get owner for PID {}", pid);
                }
            } else {
                warn!("Failed to get command string for PID {}", pid);
            }
        }

        previous_pids = current_pids;

        tokio::time::sleep(Duration::from_secs(3)).await;
    }
}

/// Retrieves the PIDs of currently running processes from the `/proc` directory.
/// This function reads the `/proc` directory and filters entries that represent PIDs.
///
/// # Returns
///
/// * `Ok(HashSet<String>)` containing the PIDs of the currently running processes.
/// * `Err(anyhow::Error)` if there's an error reading the directory.
fn get_running_processes() -> Result<HashSet<String>> {
    let mut pids = HashSet::new();

    for entry in fs::read_dir("/proc")?.filter_map(Result::ok) {
        if let Some(pid_str) = entry.file_name().to_str() {
            if pid_str.chars().all(char::is_numeric) {
                pids.insert(pid_str.to_string());
            }
        }
    }

    Ok(pids)
}

/// Retrieves the command line of a process based on its PID from `/proc/{pid}/cmdline`.
/// This function reads the `cmdline` file of a given process and returns the command line as a string.
///
/// # Parameters
///
/// * `pid` - The PID of the process whose command line is to be fetched.
///
/// # Returns
///
/// * `Some<String>` containing the command line if successful.
/// * `None` if there's an error or the command line could not be read.
fn get_command_for_pid(pid: &str) -> Option<String> {
    let cmdline_path = format!("/proc/{}/cmdline", pid);
    fs::read_to_string(cmdline_path)
        .ok()
        .map(|cmd| cmd.replace("\0", " "))
}

/// Retrieves the owner (username) of a process based on its PID.
/// This function uses `/proc/[pid]/status` to fetch the UID of the process and maps it to a username.
///
/// # Parameters
///
/// * `pid` - The PID of the process whose owner is to be fetched.
///
/// # Returns
///
/// * `Some<String>` containing the username of the process owner.
/// * `None` if the username could not be resolved.
fn get_owner_for_pid(pid: &str) -> Option<String> {
    let status_path = format!("/proc/{}/status", pid);
    let mut file = fs::File::open(status_path).ok()?;

    let mut contents = String::new();
    file.read_to_string(&mut contents).ok()?;

    let uid_line = contents.lines().find(|line| line.starts_with("Uid:"))?;
    let uid_str = uid_line.split_whitespace().nth(1)?;

    if let Ok(uid) = uid_str.parse::<u32>() {
        if let Ok(user) = User::from_uid(Uid::from_raw(uid)) {
            if let Some(u) = user {
                return Some(u.name.to_string());
            } else {
                return Some("root".to_string());
            }
        }
    }

    None
}
