<div align="center">

# **W8**

![Rust](https://img.shields.io/badge/MSRV-1.96.0-blue?style=flat-square&logo=rust&logoColor=white)
![CI](https://github.com/dev-er1/w8/actions/workflows/ci.yml/badge.svg)
[![License](https://img.shields.io/badge/License-Apache--2.0-blue?style=flat-square)](LICENSE)

**[Contributing](CONTRIBUTING.md) | [Architecture](docs/Architecture/Architecture.md) | [NB Format](docs/File-Format/File-Format.md) | [CoC](CODE_OF_CONDUCT.md) | [License](LICENSE) | [Changelog](CHANGELOG.md)**

</div>

**W8** — a register-based virtual machine with 64-bit registers and a JIT compiler (currently for x64 only).

## Preview
Donut:
![](gifs/donut.gif)

## Installation
There are several ways to get W8 — choose the one that suits you:

### 1. Pre-built binary from GitHub Releases
Download the archive for your platform from the [latest release](https://github.com/dev-er1/w8/releases/latest) page and add
the path to the `w8c` or `w8c.exe` binary to your `PATH`.

### 2. Building from source
Requires [Rust](https://www.rust-lang.org/) version **1.96.0** or later:

```sh
git clone https://github.com/dev-er1/w8.git
cd wdt/wdc
cargo build --release
```
The binary will appear at `target/release`. To use it from anywhere, add this path to your `PATH` or copy the binary to a convenient directory.
