use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::fs;
use std::fs::File;
use std::io::Read;
use std::path::Path;

use crate::{debug, trace, warn};

/// Represents the configuration for a single process.
/// This configuration includes details like the process name, owner, binary path, nice value,
/// and matching configuration.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct ProcessConfig {
    /// The name of the process.
    pub name: String,
    /// The owner of the process (optional).
    pub owner: Option<String>,
    /// The path to the binary of the process.
    pub bin: String,
    /// The nice value to set for the process.
    pub nice: i32,
    /// The configuration for matching the process.
    pub matcher: MatcherConfig,
}

/// Represents the configuration used to match a process.
/// This includes details on how the process is identified.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct MatcherConfig {
    /// The type of matching (e.g., "exact", "regex", etc.).
    pub r#type: String,
    /// The string to match against the process (optional).
    pub match_string: Option<String>,
    /// Whether to strip the path from the binary name before matching (optional).
    pub strip_path: Option<bool>,
}

/// Represents the overall configuration, which consists of a list of process configurations.
#[derive(Deserialize, Serialize, Debug, Default)]
pub struct Config {
    /// A list of process configurations.
    pub process: Vec<ProcessConfig>,
}

impl Config {
    /// Loads the global configuration from a system-wide configuration file.
    ///
    /// # Returns
    ///
    /// * `Ok(Config)` containing the global configuration if successful.
    /// * `Err(anyhow::Error)` if an error occurs during file reading or parsing.
    pub fn load_global() -> Result<Self> {
        let path = Path::new("/etc/reniced/config.yaml");
        trace!("Loading global configuration from {}", path.display());
        let config =
            Self::load_config_from_file(path).context("Failed to load global configuration")?;
        trace!("Successfully loaded global configuration");
        Ok(config)
    }

    /// Loads and merges all configurations:
    /// - The global configuration from `/etc/reniced/config.yaml`.
    /// - Local configurations from each user's home directory (if accessible).
    ///
    /// # Returns
    ///
    /// * `Ok(Config)` containing the merged configuration if successful.
    /// * `Err(anyhow::Error)` if any errors occur during configuration loading or merging.
    pub fn load_all() -> Result<Self> {
        trace!("Loading all configurations (global and local)");
        let global_config = Self::load_global().unwrap_or_else(|err| {
            debug!("Failed to load global configuration: {}", err);
            Config::default()
        });

        let home_dirs = match get_home_directories() {
            Ok(dirs) => dirs,
            Err(err) => {
                warn!("Failed to retrieve home directories: {}", err);
                return Err(err);
            }
        };

        let merged_config = home_dirs
            .into_iter()
            .filter_map(|user| match load_and_prepare_local_config(&user) {
                Ok(local_config) => {
                    trace!("Successfully loaded local configuration for user: {}", user);
                    Some(local_config)
                }
                Err(err) => {
                    warn!(
                        "Failed to load local configuration for user {}: {}",
                        user, err
                    );
                    None
                }
            })
            .fold(global_config, Self::merge);

        debug!("Successfully loaded and merged all configurations");
        Ok(merged_config)
    }

    /// Loads the local configuration specific to a user from their home directory.
    ///
    /// # Arguments
    ///
    /// * `user` - The username for which the local configuration should be loaded.
    ///
    /// # Returns
    ///
    /// * `Ok(Config)` containing the local configuration if successful.
    /// * `Err(anyhow::Error)` if an error occurs during file reading or parsing.
    pub fn load_local(user: &str) -> Result<Self> {
        if user == "lost+found" {
            return Ok(Config::default());
        }
        let config_path = format!("/home/{}/.reniced/config.yaml", user);
        let path = Path::new(&config_path);
        debug!("Loading local configuration for user: {}", user);
        Self::load_config_from_file(path)
    }
    /// Loads a configuration from a specified YAML file.
    ///
    /// # Arguments
    ///
    /// * `path` - A reference to a `Path` pointing to the YAML configuration file.
    ///
    /// # Returns
    ///
    /// * `Ok(Config)` containing the parsed configuration if successful.
    /// * `Err(anyhow::Error)` if an error occurs during file reading or parsing.
    fn load_config_from_file(path: &Path) -> Result<Self> {
        debug!("Reading configuration file: {}", path.display());
        let mut file = File::options().read(true).write(false).open(path)?;
        let mut content = String::new();
        file.read_to_string(&mut content)?;
        trace!("Successfully read file: {}", path.display());
        let config: Config = serde_yaml::from_str(&content)
            .with_context(|| format!("Failed to parse YAML file: {}", path.display()))?;
        trace!(
            "Successfully parsed configuration from file: {}",
            path.display()
        );
        Ok(config)
    }

    /// Merges two configurations: the global configuration and the local configuration.
    ///
    /// This function combines the process configurations from both global and local configs.
    /// If a process with the same name exists in both, the local configuration overwrites
    /// the corresponding fields in the global configuration. Processes present only in the local
    /// configuration are added to the merged result.
    ///
    /// # Arguments
    ///
    /// * `global` - The global `Config` object.
    /// * `local` - The local `Config` object.
    ///
    /// # Returns
    ///
    /// * A new `Config` object representing the merged configuration.
    pub fn merge(global: Config, local: Config) -> Self {
        debug!("Merging configurations");
        let mut merged_config = global;

        for local_process in local.process {
            if let Some(existing_process) = merged_config
                .process
                .iter_mut()
                .find(|p| p.name == local_process.name)
            {
                trace!(
                    "Overwriting existing process configuration: {}",
                    local_process.name
                );
                existing_process.owner = local_process.owner;
                existing_process.nice = local_process.nice;
                existing_process.matcher = local_process.matcher;
            } else {
                trace!("Adding new process configuration: {}", local_process.name);
                merged_config.process.push(local_process);
            }
        }

        debug!("Successfully merged configurations");
        merged_config
    }
}

/// Retrieves a list of home directories by reading the `/home` directory.
///
/// # Returns
///
/// * `Ok(Vec<String>)` containing the usernames of home directories found.
/// * `Err(anyhow::Error)` if an error occurs while reading the directory.
fn get_home_directories() -> Result<Vec<String>> {
    let base_home_dir = Path::new("/home");
    debug!(
        "Retrieving home directories from {}",
        base_home_dir.display()
    );
    let entries = fs::read_dir(base_home_dir)
        .with_context(|| format!("Failed to read home directory: {}", base_home_dir.display()))?;

    let users: Vec<String> = entries
        .filter_map(|entry| {
            let entry = entry.ok()?;
            let path = entry.path();
            path.file_name()?.to_str().map(|s| s.to_string())
        })
        .collect();

    debug!("Found {} home directories", users.len());
    Ok(users)
}

/// Loads a local configuration for a specific user and ensures the `owner` field is set.
///
/// # Arguments
///
/// * `user` - The username for which the local configuration is being loaded.
///
/// # Returns
///
/// * `Ok(Config)` containing the user's local configuration with the `owner` field updated.
/// * `Err(anyhow::Error)` if an error occurs during configuration loading.
fn load_and_prepare_local_config(user: &str) -> Result<Config> {
    debug!(
        "Loading and preparing local configuration for user: {}",
        user
    );
    let mut local_config = Config::load_local(user)?;
    for process in &mut local_config.process {
        if process.owner.is_none() {
            trace!("Setting owner for process {} to {}", process.name, user);
            process.owner = Some(user.to_string());
        }
    }
    debug!(
        "Successfully prepared local configuration for user: {}",
        user
    );
    Ok(local_config)
}
