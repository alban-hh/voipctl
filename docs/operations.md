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

