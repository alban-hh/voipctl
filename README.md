# voipctl

[![CI](https://github.com/alban-hh/voipctl/actions/workflows/ci.yml/badge.svg)](https://github.com/alban-hh/voipctl/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/Rust-1.88%2B-000000?logo=rust)](https://www.rust-lang.org/)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](LICENSE)

**CLI-first administration for outbound Asterisk servers.**

Adding a customer should not require coordinating several configuration files by hand. voipctl turns customer policies, extensions, trunks, and caller ID pools into validated Asterisk configuration, with explicit deployment and recovery.

## Capabilities

- Multiple customers and credential-based SIP trunks.
- Destination allowlists, global blocked prefixes, and optional customer source locks.
- Fixed or pooled caller IDs, `10` / `11` selection, and national number normalization.
- Customer, trunk, and per-number concurrency limits.
- Staged changes, credential-free previews, drift detection, checkpoints, and rollback.
- Legacy configuration import, existing MariaDB CDR queries, JSON output, and shell completion.

## Install

Requires Rust 1.88 or newer to build. Live administration targets Linux, systemd, and an existing Asterisk 22 installation. macOS supports offline configuration work.

```sh
git clone https://github.com/alban-hh/voipctl.git
cd voipctl
cargo build --release --locked
sudo install -m 0755 target/release/voipctl /usr/local/sbin/voipctl
```

## Start offline

```sh
voipctl_root="$(mktemp -d)"
voipctl --root "$voipctl_root" init --domain pbx.example.com
voipctl --root "$voipctl_root" --help
```

Changes are staged until `apply`. A custom `--root` never activates system services.

[Operations](docs/operations.md) · [Migration](docs/migration.md) · [Design](docs/design.md) · [Contributing](CONTRIBUTING.md)

## Validation

```sh
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
```

Pre-1.0 software. Validation currently covers Rust unit tests with temporary files and simulated services. Live Asterisk behavior and carrier interoperability have not been verified for this release.

voipctl manages configuration for a dedicated outbound server. It does not install Asterisk, provision a carrier account, manage cloud firewall rules, or provide billing.

## License

[MIT](LICENSE).
