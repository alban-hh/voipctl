# Design

```text
CLI → validated state → deterministic configuration → deployment transaction
                             ↓                              ↓
                        redacted plan                activation + verification
                                                            ↓
                                                    recovery on failure
```

## Boundaries

| Module | Responsibility |
| --- | --- |
| `model` | Versioned configuration, references, credentials, and policy validation |
| `admin` | Customer, extension, trunk, pool, and policy mutations |
| `dialing` | Offline destination normalization and policy explanation |
| `render` | Asterisk and optional fail2ban configuration |
| `storage` | Private state, atomic file replacement, and writer locking |
| `transaction` | Checkpoints, drift detection, journals, and recovery |
| `services` | Bounded process execution and live service verification |
| `migrate` | Import of the original configuration format |

## Changes

Mutations run under an exclusive workspace lock. The entire prospective state is validated before replacing the root-only state file. No shell is used to execute service commands.

Deployment snapshots existing managed files, durably writes a recovery journal, replaces files, activates services, and verifies the result. Failures restore the prior files and attempt reactivation. A failed recovery leaves the journal in place and blocks further changes.

File replacement is atomic per file, not across the whole filesystem. The journal provides recovery after interruption. Recovery cannot guarantee service availability if the host, disk, or underlying service is broken.

Call admission uses an Asterisk lock around customer, trunk, and caller ID group checks and reservations. These limits apply to one Asterisk instance; they are not distributed across a cluster.

## Scope

Credential-based UDP PJSIP trunks, one registration per extension, fixed destination policies, and dedicated outbound servers are supported. TLS/SRTP, inbound routing, carrier failover, distributed limits, billing, and automatic Asterisk installation are not implemented.
