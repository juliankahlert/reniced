# reniced
A daemon to monitor and adjust nice values of processes based on configuration

## Features

- **Process Monitoring**: Continuously observes starting processes to identify those matching specified criteria.
- **Dynamic Priority Adjustment**: Automatically modifies the nice values of processes according to configuration rules.
- **Customizable Configuration**: Allows users to define rules for adjusting process priorities using regular expressions.
- **Systemd Integration**: Includes a systemd service file for easy management and integration with system startup.

## Project Structure

```
reniced.git/
├── src/
│   ├── main.rs         # Entry point for the application.
│   ├── config.rs       # Parsing and managing YAML configuration.
│   ├── monitor.rs      # Monitoring processes via procfs.
│   ├── matcher.rs      # Implementing the process matching logic.
│   ├── adjuster.rs     # Logic for adjusting nice values.
│   ├── logger.rs       # Logging initialization and setup.
├── Cargo.toml          # Dependency and metadata file.
└── README.md           # Documentation for users and developers.
```

## Installation

### Prerequisites

Ensure that your system has the following dependencies installed:

- **Rust**: Install Rust by following the instructions at [rust-lang.org](https://www.rust-lang.org/tools/install).
- **`procfs` Library**: Utilized for process monitoring within the application.

### Building from Source

1. **Clone the Repository**:

   ```bash
   git clone https://github.com/juliankahlert/reniced.git
   cd reniced
   ```

2. **Build the Project**:

   Use `cargo` to build the project:

   ```bash
   cargo build --release
   ```

3. **Set Capabilities**:

   Grant the necessary capabilities to allow `reniced` to adjust process priorities:

   ```bash
   sudo setcap cap_sys_nice=eip /path/to/reniced
   ```

### RPM Package

For RPM-based distributions, you can build and install an RPM package:

1. **Install Dependencies**:

   ```bash
   sudo dnf install rpmdevtools
   cargo install cargo-generate-rpm
   ```

2. **Build the RPM**:

   ```bash
   make && make rpm
   ```

3. **Install the RPM**:

   ```bash
   sudo make install
   ```

## Configuration

`reniced` uses a YAML configuration file to define rules for adjusting process priorities. Place the global configuration file at `/etc/reniced/config.yaml`. Place the user configuration file at `/home/<user>/.reniced/config.yaml`.

### Sample Configuration

```yaml
---
process:
- name:  "Global Process"
  bin: foo
  nice: -15
  matcher:
    type: simple
    strip_path: true
```

## Usage

Start `reniced` using the provided systemd service:

```bash
sudo systemctl enable reniced.service
sudo systemctl start reniced.service
```

To check the status of the service:

```bash
sudo systemctl status reniced.service
```

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
