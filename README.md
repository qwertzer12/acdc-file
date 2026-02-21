# ACDC

> Automated Creator for Docker Compose

[![Rust](https://img.shields.io/badge/rust-stable-orange.svg)](https://rust-lang.org/)
[![License](https://img.shields.io/badge/license-GPLv3-blue.svg)](#license)

## Overview

A Rust-based TUI for creating Docker Compose Files.
Created due to a desire for a tool automating this process.
Features a searcher for Docker Images.

## Quick Start

### Prerequisites

- Rust toolchain (`rustup`, `cargo`)
- Optional: Docker

### Build

```bash
cargo build --release
```

### Run

```bash
cargo run --release
```

## Usage

```bash
# Example
acdc --help
```

### Common Commands

| Command | Description |
| --- | --- |
| `acdc --help` | Show available commands/options |
| `acdc <args>` | Run the main flow |

## License

This project is licensed under the GNU General Public License v3.0 (GPLv3).

---

If useful, add screenshots/GIFs under a `docs/` folder and link them in this README.
