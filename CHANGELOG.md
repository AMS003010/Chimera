# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.6.0] - 2025-06-23

### Added
- Cross-platform binary releases for Linux, macOS, and Windows
- Automated GitHub Actions workflow for building and releasing binaries
- Debian package support with automated .deb generation
- Mock API functionality with comprehensive endpoint support
- Command-line interface with clap for easy configuration
- Colored terminal output for better user experience
- JSON response generation with fake data support
- Local IP address detection and display
- Multi-threaded request handling with rayon
- HTTP server implementation using axum and hyper

### Changed
- Initial public release with core functionality

### Security
- Built with latest stable Rust toolchain for security best practices

## [0.5.0] - 2025-06-XX

### Added
- Core mock API server implementation
- Basic HTTP endpoint handling
- JSON response formatting

### Changed
- Internal architecture improvements

## [0.4.0] - 2025-05-XX

### Added
- Initial project structure
- Basic CLI argument parsing
- Core dependencies setup

---

## Release Links

- [0.6.0]: https://github.com/ams003010/chimera/releases/tag/v0.6.0

## Installation

### From GitHub Releases

Download the appropriate binary for your platform from the [releases page](https://github.com/ams003010/chimera/releases).

### Debian/Ubuntu

```bash
wget https://github.com/ams003010/chimera/releases/download/v0.6.0/chimera_0.6.0_amd64.deb
sudo dpkg -i chimera_0.6.0_amd64.deb
```

### From Source

```bash
git clone https://github.com/ams003010/chimera.git
cd chimera
cargo build --release
```