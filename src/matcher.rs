use crate::config::{Config, ProcessConfig};

/// A struct that handles matching processes against the configuration.
///
/// The `ProcessMatcher` uses process configuration to check if a given process matches
/// specified criteria (e.g., binary name, command-line arguments).
pub struct ProcessMatcher<'a> {
    config: &'a Config,
}

impl<'a> ProcessMatcher<'a> {
    /// Creates a new `ProcessMatcher` instance with the provided configuration.
    ///
    /// # Arguments
    ///
    /// * `config` - A reference to the loaded configuration (`Config`).
    ///
    /// # Returns
    ///
    /// * A new instance of `ProcessMatcher`.
    pub fn new(config: &'a Config) -> Self {
        ProcessMatcher { config }
    }

    /// Matches a command string against the configuration's process settings.
    ///
    /// # Arguments
    ///
    /// * `command` - The command string to match against the configuration.
    /// * `process_owner` - The owner of the process being checked (e.g., username).
    ///
    /// # Returns
    ///
    /// * `Some(&ProcessConfig)` if the command matches a process configuration.
    /// * `None` if no match is found.
    pub fn match_command(&self, command: &str, process_owner: &str) -> Option<&ProcessConfig> {
        for process_config in &self.config.process {
            if self.is_command_matched(command, process_owner, process_config) {
                return Some(process_config);
            }
        }
        None
    }

    /// Extracts the matching pattern based on the process configuration.
    ///
    /// # Arguments
    ///
    /// * `process_config` - A reference to a `ProcessConfig` struct.
    ///
    /// # Returns
    ///
    /// * A `String` representing the match pattern. If `match_string` is set, it is returned.
    ///   Otherwise, the `bin` value is used with a trailing space.
    fn get_pattern(&self, process_config: &ProcessConfig) -> String {
        if let Some(match_string) = &process_config.matcher.match_string {
            match_string.clone()
        } else {
            format!("{} ", process_config.bin)
        }
    }

    /// Strips the path from the command if the `strip_path` option is enabled.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The full command string.
    /// * `pattern` - The pattern to locate in the command string.
    ///
    /// # Returns
    ///
    /// * A `String` representing the command with the path stripped (if applicable).
    fn strip_path_from_command(&self, cmd: &str, pattern: &String) -> String {
        if let Some(first_space_index) = cmd.find(pattern) {
            let rest_of_cmd = &cmd[first_space_index..].trim_start();
            format!("{}", rest_of_cmd)
        } else {
            cmd.to_string()
        }
    }

    /// Prepares the command for matching by optionally stripping the path.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The full command string.
    /// * `pattern` - The pattern used for matching.
    /// * `matcher` - A reference to the `MatcherConfig` specifying match settings.
    ///
    /// # Returns
    ///
    /// * A `String` representing the prepared command string.
    fn prepare_command(
        &self,
        cmd: &str,
        pattern: &String,
        matcher: &crate::config::MatcherConfig,
    ) -> String {
        let strip = matcher.strip_path.unwrap_or(false);

        if strip {
            self.strip_path_from_command(cmd, pattern)
        } else {
            cmd.to_string()
        }
    }

    /// Matches a command against a simple matching type.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The full command string.
    /// * `pattern` - The pattern to match against.
    /// * `matcher` - A reference to the `MatcherConfig` specifying match settings.
    ///
    /// # Returns
    ///
    /// * `true` if the command matches the pattern.
    /// * `false` otherwise.
    fn match_simple(
        &self,
        cmd: &str,
        pattern: &String,
        matcher: &crate::config::MatcherConfig,
    ) -> bool {
        let cmd_to_check = self.prepare_command(cmd, pattern, matcher);
        cmd_to_check.starts_with(pattern.as_str())
    }

    /// Checks if a command matches the given process configuration.
    ///
    /// # Arguments
    ///
    /// * `cmd` - The full command string to check.
    /// * `process_owner` - The owner of the process being checked.
    /// * `process_config` - A reference to the `ProcessConfig` containing match criteria.
    ///
    /// # Returns
    ///
    /// * `true` if the command matches the process configuration.
    /// * `false` otherwise.
    fn is_command_matched(
        &self,
        cmd: &str,
        process_owner: &str,
        process_config: &ProcessConfig,
    ) -> bool {
        if let Some(config_owner) = &process_config.owner {
            if config_owner != process_owner {
                return false;
            }
        }

        let matcher = &process_config.matcher;
        let pattern = self.get_pattern(process_config);

        match matcher.r#type.as_str() {
            "simple" => self.match_simple(cmd, &pattern, matcher),
            _ => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::{Config, MatcherConfig, ProcessConfig};

    /// Helper function to create a sample `ProcessConfig` for testing.
    /// This is used to ensure consistency across tests.
    fn create_test_process_config() -> ProcessConfig {
        ProcessConfig {
            name: String::from("test_process"),
            owner: Some(String::from("test_user")),
            bin: String::from("/usr/bin/test"),
            nice: 10,
            matcher: MatcherConfig {
                r#type: String::from("simple"),
                match_string: Some(String::from("test_process")),
                strip_path: Some(true),
            },
        }
    }

    /// Tests that `get_pattern` correctly retrieves the `match_string` from the configuration
    /// when it is explicitly set.
    ///
    /// This ensures that the function prioritizes the explicit `match_string` field
    /// over other fallback mechanisms.
    #[test]
    fn test_get_pattern_with_match_string() {
        let binding = Config::default();
        let process_config = create_test_process_config();
        let matcher = ProcessMatcher::new(&binding);

        let pattern = matcher.get_pattern(&process_config);
        assert_eq!(pattern, "test_process");
    }

    /// Tests that `get_pattern` falls back to the binary path (`bin` field) when `match_string` is not set.
    ///
    /// This ensures that the function has a robust fallback mechanism for pattern matching.
    #[test]
    fn test_get_pattern_without_match_string() {
        let binding = Config::default();
        let mut process_config = create_test_process_config();
        process_config.matcher.match_string = None;
        let matcher = ProcessMatcher::new(&binding);

        let pattern = matcher.get_pattern(&process_config);
        assert_eq!(pattern, "/usr/bin/test ");
    }

    /// Tests that `strip_path_from_command` correctly identifies and extracts the relevant portion
    /// of the command when the specified pattern exists in the input command.
    ///
    /// This verifies that path stripping works as intended for valid patterns.
    #[test]
    fn test_strip_path_from_command_with_valid_pattern() {
        let binding = Config::default();
        let matcher = ProcessMatcher::new(&binding);
        let cmd = "/usr/bin/test_process --arg value";
        let pattern = String::from("test_process ");

        let stripped = matcher.strip_path_from_command(cmd, &pattern);
        assert_eq!(stripped, "test_process --arg value");
    }

    /// Tests that `strip_path_from_command` returns the command unchanged
    /// when the specified pattern is not found.
    ///
    /// This ensures that the function gracefully handles cases where no matching pattern exists.
    #[test]
    fn test_strip_path_from_command_with_invalid_pattern() {
        let binding = Config::default();
        let matcher = ProcessMatcher::new(&binding);
        let cmd = "/usr/bin/other_process --arg value";
        let pattern = String::from("test_process");

        let stripped = matcher.strip_path_from_command(cmd, &pattern);
        assert_eq!(stripped, cmd);
    }

    /// Tests that `prepare_command` correctly prepares the command by stripping the path
    /// when the `strip_path` field is set to `true` in the matcher configuration.
    ///
    /// This verifies that path stripping behavior is configurable.
    #[test]
    fn test_prepare_command_with_strip_path() {
        let binding = Config::default();
        let matcher = ProcessMatcher::new(&binding);
        let cmd = "/usr/bin/test_process --arg value";
        let pattern = String::from("test_process");
        let mut matcher_config = MatcherConfig::default();
        matcher_config.strip_path = Some(true);

        let prepared_cmd = matcher.prepare_command(cmd, &pattern, &matcher_config);
        assert_eq!(prepared_cmd, "test_process --arg value");
    }

    /// Tests that `prepare_command` returns the command unchanged
    /// when the `strip_path` field is not set or set to `false` in the matcher configuration.
    ///
    /// This ensures that the function does not modify commands unnecessarily.
    #[test]
    fn test_prepare_command_without_strip_path() {
        let binding = Config::default();
        let matcher = ProcessMatcher::new(&binding);
        let cmd = "/usr/bin/test_process --arg value";
        let pattern = String::from("test_process");
        let matcher_config = MatcherConfig::default();

        let prepared_cmd = matcher.prepare_command(cmd, &pattern, &matcher_config);
        assert_eq!(prepared_cmd, cmd);
    }

    /// Tests that `match_simple` correctly identifies a command as matching
    /// when the `strip_path` is enabled and the command starts with the expected pattern.
    ///
    /// This ensures that simple matching works as expected for valid inputs.
    #[test]
    fn test_match_simple_with_matching_command() {
        let binding = Config::default();
        let matcher = ProcessMatcher::new(&binding);
        let cmd = "/usr/bin/test_process --arg value";
        let pattern = String::from("test_process");
        let mut matcher_config = MatcherConfig::default();
        matcher_config.strip_path = Some(true);

        let is_matched = matcher.match_simple(cmd, &pattern, &matcher_config);
        assert!(is_matched);
    }

    /// Tests that `match_simple` correctly identifies a command as non-matching
    /// when the command does not start with the expected pattern.
    ///
    /// This ensures that the function does not produce false positives.
    #[test]
    fn test_match_simple_with_non_matching_command() {
        let binding = Config::default();
        let matcher = ProcessMatcher::new(&binding);
        let cmd = "/usr/bin/other_process --arg value";
        let pattern = String::from("test_process");
        let matcher_config = MatcherConfig::default();

        let is_matched = matcher.match_simple(cmd, &pattern, &matcher_config);
        assert!(!is_matched);
    }
}
