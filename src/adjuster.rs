use nix::libc;
use procfs::process::Process;

use crate::config::{Config, ProcessConfig};
use crate::{debug, error, info, warn};

/// The `Adjuster` struct is responsible for managing and adjusting the nice values
/// for processes. It interacts with the system to check the current nice value
/// of a process and adjust it according to the configuration.
pub struct Adjuster<'a> {
    _config: &'a Config,
}

impl<'a> Adjuster<'a> {
    /// Creates a new instance of `Adjuster` with the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - A reference to the `Config` object which contains the desired nice values for processes.
    ///
    /// # Returns
    ///
    /// Returns a new `Adjuster` instance initialized with the provided configuration.
    pub fn new(config: &'a Config) -> Self {
        Adjuster { _config: config }
    }

    /// This is the main function that checks the current nice value of a process
    /// and adjusts it if necessary. Logs the process and any issues along the way.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID (PID) of the process to check and adjust.
    /// * `process_config` - The `ProcessConfig` object that defines the expected nice value.
    ///
    /// # Description
    ///
    /// This function is responsible for initiating the checking and adjustment of the nice value.
    /// It logs the start and end of the process, handles any errors, and provides detailed debugging information.
    pub fn check_and_adjust_nice_value(&self, pid: i32, process_config: &ProcessConfig) {
        debug!(
            "Starting check and adjust for PID {} with expected nice value {}",
            pid, process_config.nice
        );

        if let Err(e) = self.try_check_and_adjust_nice_value(pid, process_config) {
            warn!(
                "Failed to check and adjust nice value for PID {}: {}",
                pid, e
            );
        }

        debug!(
            "Finished check and adjust for PID {} with expected nice value {}",
            pid, process_config.nice
        );
    }

    /// Tries to check the current nice value of the process and adjusts it if necessary.
    /// If there is an error at any point, it propagates the error.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID (PID) of the process.
    /// * `process_config` - The configuration that contains the expected nice value for the process.
    ///
    /// # Returns
    ///
    /// A `Result` which is `Ok` if everything was successful or `Err` if an error occurred.
    fn try_check_and_adjust_nice_value(
        &self,
        pid: i32,
        process_config: &ProcessConfig,
    ) -> Result<(), String> {
        debug!("Fetching process details for PID {}", pid);
        let process = self.get_process(pid)?;

        debug!("Fetching current nice value for PID {}", pid);
        let current_nice = self.get_current_nice_value(&process)?;

        let expected_nice = process_config.nice;
        debug!(
            "Current nice value for PID {}: {}, Expected nice value: {}",
            pid, current_nice, expected_nice
        );

        if current_nice != expected_nice {
            self.log_nice_mismatch(process_config, pid, current_nice, expected_nice);
            debug!("Adjusting nice value for PID {}", pid);
            self.adjust_nice_value(pid, expected_nice)?;
        } else {
            self.log_nice_match(process_config, pid, current_nice);
        }

        Ok(())
    }

    /// Retrieves the process for a given PID.
    ///
    /// # Arguments
    ///
    /// * `pid` - The process ID of the process to retrieve.
    ///
    /// # Returns
    ///
    /// A `Result` containing either the `Process` object or an error message if the process cannot be found.
    fn get_process(&self, pid: i32) -> Result<Process, String> {
        debug!("Attempting to access process for PID {}", pid);
        Process::new(pid).map_err(|_| format!("Could not access process for PID {}", pid))
    }

    /// Fetches the current nice value of a given process.
    ///
    /// # Arguments
    ///
    /// * `process` - The `Process` object from which the nice value is fetched.
    ///
    /// # Returns
    ///
    /// A `Result` containing the nice value (`i32`) of the process or an error message.
    fn get_current_nice_value(&self, process: &Process) -> Result<i32, String> {
        debug!("Fetching stat information for PID {}", process.pid);

        process
            .stat()
            .map(|stat| stat.nice as i32) // Cast stat.nice (i64) to i32
            .map_err(|_| format!("Could not access stat information for PID {}", process.pid))
    }

    /// Logs a message when there is a mismatch between the current and expected nice values.
    ///
    /// # Arguments
    ///
    /// * `process_config` - The process configuration that contains the expected nice value.
    /// * `pid` - The PID of the process.
    /// * `current_nice` - The current nice value of the process.
    /// * `expected_nice` - The expected nice value for the process.
    ///
    /// # Description
    ///
    /// This function logs a warning indicating that the current nice value does not match the expected nice value.
    fn log_nice_mismatch(
        &self,
        process_config: &ProcessConfig,
        pid: i32,
        current_nice: i32,
        expected_nice: i32,
    ) {
        info!(
            "Process '{}' (PID: {}) has a nice value of {} but expected {}. Adjusting...",
            process_config.name, pid, current_nice, expected_nice
        );
    }

    /// Logs a message when the current nice value matches the expected nice value.
    ///
    /// # Arguments
    ///
    /// * `process_config` - The process configuration that contains the expected nice value.
    /// * `pid` - The PID of the process.
    /// * `current_nice` - The current nice value of the process.
    ///
    /// # Description
    ///
    /// This function logs an informational message indicating that the process already has the correct nice value.
    fn log_nice_match(&self, process_config: &ProcessConfig, pid: i32, current_nice: i32) {
        debug!(
            "Process '{}' (PID: {}) already has the correct nice value of {}",
            process_config.name, pid, current_nice
        );
    }

    /// Attempts to adjust the nice value of a process using the `setpriority` system call.
    ///
    /// # Arguments
    ///
    /// * `pid` - The PID of the process to adjust.
    /// * `nice_value` - The desired nice value to set for the process.
    ///
    /// # Returns
    ///
    /// A `Result` indicating whether the nice value was successfully adjusted (`Ok`) or if there was an error (`Err`).
    ///
    /// # Description
    ///
    /// This function attempts to adjust the nice value for a given process using the `libc::setpriority` function.
    /// If successful, an informational log is created, and if it fails, an error log is generated with the error message.
    fn adjust_nice_value(&self, pid: i32, nice_value: i32) -> Result<(), String> {
        debug!(
            "Attempting to set nice value for PID {} to {}",
            pid, nice_value
        );

        let result = unsafe { libc::setpriority(libc::PRIO_PROCESS, pid as u32, nice_value) };

        if result == 0 {
            info!(
                "Successfully adjusted nice value for PID {} to {}",
                pid, nice_value
            );
            Ok(())
        } else {
            let error_message = format!(
                "Failed to adjust nice value for PID {}: {}",
                pid,
                std::io::Error::last_os_error()
            );

            error!("{}", error_message);
            Err(error_message)
        }
    }
}
