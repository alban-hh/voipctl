# Legacy migration

The importer reads the original Python CLI's directory structure:

```text
/etc/voip/
  customers/*.conf
  cidpools/*.txt
  secrets/telnyx.conf
  telnyx.conf
  whitelist.conf
  blocked_prefixes.conf
```

## Import and review

```sh
sudo voipctl migrate --from /etc/voip --domain pbx.example.com
sudo voipctl show
sudo voipctl check
sudo voipctl plan
```

Migration requires an empty voipctl workspace. It preserves SIP credentials, extension numbers, caller ID choices, destination policies, limits, and customer source locks. Source files are not modified.

The imported carrier is named `telnyx`. Legacy admin entries are not converted into host firewall rules. Customer limits and call duration are preserved; the new trunk concurrency limit starts at 100. Review it before deployment.

## Activate

```sh
sudo voipctl apply --adopt-existing --restart
```

Use a maintenance window. The old top-level Asterisk files are backed up before replacement. Transport defaults are IPv4 UDP on port 5060, with RTP ports 10000–20000; customize them using `server` if the existing installation differs.

Unlike the original script, mutation commands stage changes. Run `plan` and `apply` explicitly. Pool imports and edits also require `apply`; there is no background watcher.

For a prior checkpoint, use `history` followed by `rollback ID --restart`. The previous generated files are restored; the imported desired state remains available for review.
