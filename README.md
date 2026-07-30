# voipctl

Outbound Asterisk administration from the command line.

This project started with an operator's problem: adding a customer, assigning caller IDs, limiting calls, and changing allowed destinations required coordinating several Asterisk files. A small local script solved the immediate problem. This Rust implementation turns that workflow into a testable application with explicit configuration changes and recovery.

The first supported deployment is Debian with Asterisk 22, PJSIP, and an optional existing MariaDB CDR integration. Development and offline configuration previews also work on macOS.

The project is under active development. A successful build is not a substitute for deployment acceptance tests on the target PBX.
