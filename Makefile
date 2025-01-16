# Makefile for building Rust project and generating RPM

# Project name and version
PACKAGE_NAME = reniced
ARCH ?= $(shell uname -m)

# Default target
.DEFAULT_GOAL := release

# Build options
RUSTFLAGS_DEBUG = --debug
RUSTFLAGS_RELEASE = --release

# Build the Rust project for release
release:
	@echo "Building release version..."
	cargo build --release

# Build the Rust project for debug
debug:
	@echo "Building debug version..."
	cargo build --debug

# Create an RPM package
rpm: release
	@echo "Creating RPM package..."
	cargo generate-rpm
	@echo "RPM created successfully!"

# Clean the build artifacts
clean:
	@echo "Cleaning the project..."
	cargo clean

# Create an package
# Test for
# DNF -> fedora derivate -> make a RPM
# APT -> debian derivate -> make a DEB
package:
	(command -v dnf && $(MAKE) rpm) || (command -v apt && $(MAKE) deb)


# Install the RPM package
install-rpm:
	@echo "Installing RPM package..."
	rpm -i target/generate-rpm/$(PACKAGE_NAME)*.$(ARCH).rpm

# Install the DEB package
install-deb:
	@echo "Installing DEB package..."
	@echo "Not implemented"
	exit 1

# Install the package (useful if you want to install it after building)
# Test for
# DNF -> fedora derivate -> install the RPM
# APT -> debian derivate -> install the DEB
install:
	(command -v dnf && $(MAKE) install-rpm) || (command -v apt && $(MAKE) install-deb)

.PHONY: release debug rpm clean install install-rpm install-deb package
