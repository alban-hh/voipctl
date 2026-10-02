# Contributing

Open an issue describing the operator problem before proposing a significant feature. Keep changes focused and preserve compatibility with existing state files.

```sh
cargo fmt --all -- --check
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
sh -n scripts/install.sh tests/installer.sh
sh tests/installer.sh
```

Use descriptive types, names, and small modules. Keep source code free of comments; document behavior in tests and operator documentation. New failure paths need regression tests, especially around state changes, activation, and recovery.

Tests use temporary workspaces and simulated services. Do not add tests that contact a carrier, modify a live PBX, or require production credentials.

Never include real SIP credentials, customer data, call records, or configuration backups in an issue or pull request. Report suspected credential exposure or access-control defects privately through GitHub's security reporting when enabled.
