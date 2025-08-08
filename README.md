# sergw — Simple Serial ↔ TCP Gateway

A small utility that bridges a serial port to a TCP socket. It exposes the serial port as a TCP server: any connected TCP client receives bytes read from the serial port, and bytes written by a client are forwarded to the serial port.

## Features

- Auto-detect serial port when exactly one is available (or require `--serial`)
- Configurable baud, data bits, parity, stop bits
- Type-safe TCP bind address (`--host`) with sensible default
- Concurrency-safe design: single serial reader, single serial writer
- Backpressure via bounded channels; slow TCP writers are dropped
- Graceful shutdown on Ctrl-C
- Structured logs via `tracing`

## Install

- Build locally:

```bash
cargo build --release
```

- Or install to cargo bin:

```bash
cargo install --path . --locked
```

## Usage

- List ports (USB only by default):

```bash
sergw ports
sergw ports --verbose
sergw ports --all --verbose
```

- Listen (auto-pick serial if only one is present):

```bash
sergw listen --baud 115200 --host 127.0.0.1:5656
```

- Listen with explicit serial:

```bash
sergw listen --serial /dev/ttyUSB0 --baud 115200 --host 0.0.0.0:5656
```

- Serial settings examples:

```bash
sergw listen --serial /dev/ttyUSB0 --baud 57600 --data-bits eight --parity none --stop-bits one
```

Enable logs with `RUST_LOG` (examples):

```bash
RUST_LOG=info sergw listen --serial /dev/ttyUSB0
RUST_LOG=debug,sergw=trace sergw ports --verbose
```

## Makefile

Common developer commands are available:

```bash
make help
```

Key targets:
- `make build` / `make release`
- `make test` / `make clippy` / `make fmt`
- `make ports`
- `make listen SERIAL=/dev/ttyUSB0 BAUD=115200 HOST=127.0.0.1:5656`

## Architecture (short)

- One thread reads from the serial port and broadcasts to all TCP clients
- One thread serializes writes from all TCP clients to the serial port
- Per-connection threads forward TCP->serial and broadcast serial->TCP
- Backpressure is enforced using bounded channels (slow clients get dropped)

## Notes and Limitations

- No framing or protocol translation: raw bytes are forwarded as-is
- No authentication or encryption on the TCP side (consider using TLS/SSH)
- The accept loop currently uses blocking `accept`; a future improvement is to use non-blocking accept to improve shutdown responsiveness

## Systemd (example)

```ini
[Unit]
Description=sergw Serial to TCP Gateway
After=network.target

[Service]
Type=simple
ExecStart=/usr/local/bin/sergw listen --serial /dev/ttyUSB0 --baud 115200 --host 0.0.0.0:5656
Restart=on-failure
Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
```

## License

GPL-3.0-or-later. See `LICENSE`.

## Architectures

Built and tested on Linux. The project is architecture-agnostic and should
work on common Linux architectures supported by Rust, including `x86_64`,
`aarch64` (e.g., Raspberry Pi 4/5), `armv7h` (e.g., Raspberry Pi 2/3), and
`i686`. The Arch Linux PKGBUILD declares these architectures.
