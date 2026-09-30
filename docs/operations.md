# Operations

Commands below assume installation on a dedicated Linux host with Asterisk 22. Use `--root /absolute/workspace` for offline work. Configuration edits require root only when operating on `/`.

## Configure

```sh
sudo voipctl init --domain pbx.example.com
sudo voipctl trunk add telnyx --host sip.telnyx.eu
sudo voipctl trunk credentials telnyx
sudo voipctl customer add main --trunk telnyx --allow 1,355,52,508 --default-country 355
sudo voipctl pool add us +12025550100 +12025550101
sudo voipctl ext add main 101 --cid +16135550100 --pool2 us
sudo voipctl ext show main 101 --reveal
```

The numbers above are examples. Use caller IDs authorized by your carrier. Trunk credentials are entered at hidden prompts. For automation, `trunk credentials NAME --stdin` accepts a JSON object containing `username` and `password`; passwords are never command-line arguments.

Dial normally or use `10<number>` for the primary caller ID. Use `11<number>` for the alternate. International `+355…`, `00355…`, and `355…` formats work; `069…` requires `default-country=355`. Spaces and punctuation are rejected. The `1` prefix permits the entire North American Numbering Plan, not only the US and Canada.

## Deploy

```sh
sudo voipctl check
sudo voipctl plan
sudo voipctl apply --adopt-existing --restart
sudo voipctl doctor
```

The first deployment replaces `pjsip.conf`, `extensions.conf`, `rtp.conf`, `manager.conf`, `http.conf`, and `logger.conf` under `/etc/asterisk`, plus the dedicated `/etc/fail2ban/jail.d/voipctl.conf` file. Inspect the existing configuration before adoption. This workflow is not intended for a FreePBX-managed or shared inbound PBX.

Subsequent ordinary changes use `apply`. Transport, RTP, or service-setting changes require `--restart`; restarts are refused while channels are active. Use a maintenance window. `doctor` checks runtime prerequisites; it does not place calls.

The target must provide `/usr/sbin/asterisk`, `/usr/bin/systemctl`, the `asterisk` Unix group, and the standard PJSIP, dialplan, string, caller ID, CDR, timeout, random, group-count, and locking modules. In particular, enable `func_lock.so` in custom Asterisk builds.

## Limits and access

```sh
sudo voipctl customer set main --max-calls 6
sudo voipctl pool limit us 2
sudo voipctl customer set main --source-ip 203.0.113.10/32
sudo voipctl firewall --admin 203.0.113.20/32
```

Pool selection starts at a random number and checks the remaining numbers when it is full. Per-number limits apply across customers; all-full pools reject the call. Customer and trunk limits still apply. A default per-number limit can be set with `server --max-calls-per-number N`; zero means unlimited. `pool limit` sets overrides for the numbers currently in that pool.

IP access stays in the cloud firewall. Customer source locks are optional additional restrictions. Configure current carrier ranges with `trunk set --signaling CIDR --media CIDR`. `firewall` prints requirements and never changes rules. UDP SIP and RTP are unencrypted in this release.

Managed fail2ban is opt-in: `server --manage-fail2ban true`. It requires an installed, running fail2ban with the Asterisk filter and nftables action. Existing SSH jails are unchanged.

## Recovery and records

`history` lists pre-change checkpoints. `rollback ID --restart` restores generated files and records an undo checkpoint. `recover --restart` restores an interrupted transaction. Both preserve the desired configuration; inspect `plan` before applying again.

The root-only state is `/etc/voipctl/state.toml`. Root-only checkpoints are under `/var/lib/voipctl/history`; back them up securely and manage retention. Checkpoints can contain credentials. Never add either directory to Git.

`cdr --today --summary` queries an existing `asteriskcdr.cdr` table through local MariaDB authentication. The detailed view expects `id`, `calldate`, `accountcode`, `customer_ext`, `src`, `dialed`, `disposition`, `billsec`, `src_ip`, and `userfield`. Existing ODBC/CDR configuration is retained; schema provisioning is outside this release.
