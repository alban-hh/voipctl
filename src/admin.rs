use crate::{cli::*, model::*, storage::Layout};
use anyhow::{Context, Result, bail, ensure};
use serde::Serialize;
use std::{
    collections::BTreeMap,
    fs,
    io::{self, IsTerminal, Read},
};

pub fn display<T: Serialize>(value: &T, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::to_string_pretty(value)?);
    } else {
        println!(
            "{}",
            toml::to_string_pretty(value).unwrap_or(serde_json::to_string_pretty(value)?)
        );
    }
    Ok(())
}

pub fn message(text: &str, json: bool) -> Result<()> {
    if json {
        println!("{}", serde_json::json!({"message":text}));
    } else {
        println!("{text}");
    }
    Ok(())
}

pub fn edit(
    layout: &Layout,
    json: bool,
    mutation: impl FnOnce(&mut State) -> Result<()>,
) -> Result<()> {
    let _lock = layout.lock()?;
    ensure!(
        !layout.path("var/lib/voipctl/pending.json")?.exists(),
        "recover the interrupted transaction before editing configuration"
    );
    let mut state = layout.load()?;
    mutation(&mut state)?;
    layout.save(&state)?;
    message(
        "Configuration saved. Run plan, then apply to activate it.",
        json,
    )
}

fn customer_mut<'a>(state: &'a mut State, name: &str) -> Result<&'a mut Customer> {
    state
        .config
        .customers
        .get_mut(name)
        .with_context(|| format!("unknown customer {name}"))
}

pub fn customer(layout: &Layout, command: CustomerCommand, json: bool) -> Result<()> {
    match command {
        CustomerCommand::List => display(&layout.load()?.config.customers, json),
        CustomerCommand::Show { name } => display(
            layout
                .load()?
                .config
                .customers
                .get(&name)
                .context("unknown customer")?,
            json,
        ),
        other => edit(layout, json, |state| {
            match other {
                CustomerCommand::Add {
                    name,
                    trunk,
                    allow,
                    default_country,
                    max_calls,
                    max_seconds,
                    source_ips,
                } => {
                    ensure!(
                        !state.config.customers.contains_key(&name),
                        "customer already exists"
                    );
                    state.config.customers.insert(
                        name,
                        Customer {
                            trunk,
                            allowed_prefixes: prefixes(allow),
                            default_country,
                            max_calls,
                            max_call_seconds: max_seconds,
                            source_ips,
                            extensions: BTreeMap::new(),
                        },
                    );
                }
                CustomerCommand::Set {
                    name,
                    trunk,
                    allow,
                    default_country,
                    clear_default_country,
                    max_calls,
                    max_seconds,
                    source_ips,
                    clear_source_ips,
                } => {
                    let customer = customer_mut(state, &name)?;
                    if let Some(value) = trunk {
                        customer.trunk = value;
                    }
                    if let Some(value) = allow {
                        customer.allowed_prefixes = prefixes(value);
                    }
                    if let Some(value) = default_country {
                        customer.default_country = Some(value);
                    }
                    if clear_default_country {
                        customer.default_country = None;
                    }
                    if let Some(value) = max_calls {
                        customer.max_calls = value;
                    }
                    if let Some(value) = max_seconds {
                        customer.max_call_seconds = value;
                    }
                    if let Some(value) = source_ips {
                        customer.source_ips = value;
                    }
                    if clear_source_ips {
                        customer.source_ips.clear();
                    }
                }
                CustomerCommand::Remove { name, .. } => {
                    let customer = state
                        .config
                        .customers
                        .remove(&name)
                        .context("unknown customer")?;
                    for extension in customer.extensions.keys() {
                        state.secrets.extensions.remove(extension);
                    }
                }
                _ => unreachable!(),
            }
            Ok(())
        }),
    }
}

pub fn extension(layout: &Layout, command: ExtensionCommand, json: bool) -> Result<()> {
    match command {
        ExtensionCommand::List { customer } => display(
            &layout
                .load()?
                .config
                .customers
                .get(&customer)
                .context("unknown customer")?
                .extensions,
            json,
        ),
        ExtensionCommand::Show {
            customer,
            number,
            reveal,
        } => {
            let state = layout.load()?;
            let extension = state
                .config
                .customers
                .get(&customer)
                .context("unknown customer")?
                .extensions
                .get(&number)
                .context("unknown extension")?;
            display(
                &serde_json::json!({
                    "server":state.config.server.domain, "port":state.config.server.sip_port, "transport":"udp", "username":number,
                    "caller_id":extension.caller_id, "alternate_caller_id":extension.alternate_caller_id,
                    "password":if reveal { state.secrets.extensions[&number].as_str() } else { "[redacted; use --reveal]" },
                }),
                json,
            )
        }
        other => edit(layout, json, |state| {
            match other {
                ExtensionCommand::Add {
                    customer,
                    number,
                    ids,
                } => {
                    ensure!(
                        !state.secrets.extensions.contains_key(&number),
                        "extension already exists"
                    );
                    ensure!(!ids.no_cid2, "--no-cid2 is only valid for ext set");
                    let primary = ids.primary().context("provide --cid or --pool")?;
                    customer_mut(state, &customer)?.extensions.insert(
                        number.clone(),
                        Extension {
                            caller_id: primary,
                            alternate_caller_id: ids.alternate(),
                        },
                    );
                    state.secrets.extensions.insert(number, new_password());
                }
                ExtensionCommand::Set {
                    customer,
                    number,
                    ids,
                } => {
                    ensure!(
                        ids.primary().is_some() || ids.alternate().is_some() || ids.no_cid2,
                        "provide a caller ID setting"
                    );
                    let extension = customer_mut(state, &customer)?
                        .extensions
                        .get_mut(&number)
                        .context("unknown extension")?;
                    if let Some(value) = ids.primary() {
                        extension.caller_id = value;
                    }
                    if let Some(value) = ids.alternate() {
                        extension.alternate_caller_id = Some(value);
                    }
                    if ids.no_cid2 {
                        extension.alternate_caller_id = None;
                    }
                }
                ExtensionCommand::Passwd {
                    customer,
                    number,
                    password_stdin,
                } => {
                    ensure!(
                        customer_mut(state, &customer)?
                            .extensions
                            .contains_key(&number),
                        "unknown extension"
                    );
                    let password = if password_stdin {
                        read_stdin()?.trim_end_matches(['\r', '\n']).to_owned()
                    } else {
                        new_password()
                    };
                    validate_secret(&password, 16)?;
                    state.secrets.extensions.insert(number, password);
                }
                ExtensionCommand::Remove {
                    customer, number, ..
                } => {
                    customer_mut(state, &customer)?
                        .extensions
                        .remove(&number)
                        .context("unknown extension")?;
                    state.secrets.extensions.remove(&number);
                }
                _ => unreachable!(),
            }
            Ok(())
        }),
    }
}

pub fn trunk(layout: &Layout, command: TrunkCommand, json: bool) -> Result<()> {
    match command {
        TrunkCommand::List => display(&layout.load()?.config.trunks, json),
        TrunkCommand::Show { name } => display(
            layout
                .load()?
                .config
                .trunks
                .get(&name)
                .context("unknown trunk")?,
            json,
        ),
        other => edit(layout, json, |state| {
            match other {
                TrunkCommand::Add {
                    name,
                    host,
                    port,
                    max_calls,
                    signaling,
                    media,
                } => {
                    ensure!(
                        !state.config.trunks.contains_key(&name),
                        "trunk already exists"
                    );
                    state.config.trunks.insert(
                        name,
                        Trunk {
                            host,
                            port,
                            max_calls,
                            signaling,
                            media,
                        },
                    );
                }
                TrunkCommand::Set {
                    name,
                    host,
                    port,
                    max_calls,
                    signaling,
                    media,
                } => {
                    let trunk = state
                        .config
                        .trunks
                        .get_mut(&name)
                        .context("unknown trunk")?;
                    if let Some(value) = host {
                        trunk.host = value;
                    }
                    if let Some(value) = port {
                        trunk.port = value;
                    }
                    if let Some(value) = max_calls {
                        trunk.max_calls = value;
                    }
                    if let Some(value) = signaling {
                        trunk.signaling = value;
                    }
                    if let Some(value) = media {
                        trunk.media = value;
                    }
                }
                TrunkCommand::Credentials { name, stdin } => {
                    ensure!(state.config.trunks.contains_key(&name), "unknown trunk");
                    let credentials = if stdin {
                        serde_json::from_str::<Credentials>(&read_stdin()?).map_err(|_| {
                            anyhow::anyhow!(
                                "expected JSON with username and password; values omitted"
                            )
                        })?
                    } else {
                        ensure!(
                            io::stdin().is_terminal(),
                            "use --stdin for noninteractive credentials"
                        );
                        let username = rpassword::prompt_password("SIP username: ")?;
                        let password = rpassword::prompt_password("SIP password: ")?;
                        let confirm = rpassword::prompt_password("Repeat password: ")?;
                        ensure!(password == confirm, "passwords do not match");
                        Credentials { username, password }
                    };
                    state.secrets.trunks.insert(name, credentials);
                }
                TrunkCommand::Remove { name, .. } => {
                    ensure!(
                        !state.config.customers.values().any(|c| c.trunk == name),
                        "trunk is assigned to a customer"
                    );
                    state.config.trunks.remove(&name).context("unknown trunk")?;
                    state.secrets.trunks.remove(&name);
                }
                _ => unreachable!(),
            }
            Ok(())
        }),
    }
}

pub fn pool(layout: &Layout, command: PoolCommand, json: bool) -> Result<()> {
    match command {
        PoolCommand::List => display(&layout.load()?.config.pools, json),
        PoolCommand::Show { name } => display(
            layout
                .load()?
                .config
                .pools
                .get(&name)
                .context("unknown pool")?,
            json,
        ),
        PoolCommand::Export { name } => {
            let state = layout.load()?;
            let numbers = &state
                .config
                .pools
                .get(&name)
                .context("unknown pool")?
                .numbers;
            if json {
                display(numbers, true)
            } else {
                println!("{}", numbers.join("\n"));
                Ok(())
            }
        }
        other => edit(layout, json, |state| {
            match other {
                PoolCommand::Add { name, numbers } => {
                    let pool = state
                        .config
                        .pools
                        .entry(name)
                        .or_insert(Pool { numbers: vec![] });
                    for number in numbers {
                        if !pool.numbers.contains(&number) {
                            pool.numbers.push(number);
                        }
                    }
                }
                PoolCommand::Import { name, file } => {
                    let text = fs::read_to_string(file)?;
                    let mut numbers: Vec<String> = text
                        .lines()
                        .map(|s| s.split('#').next().unwrap_or("").trim())
                        .filter(|s| !s.is_empty())
                        .map(str::to_owned)
                        .collect();
                    numbers.sort();
                    numbers.dedup();
                    state.config.pools.insert(name, Pool { numbers });
                }
                PoolCommand::Remove { name, numbers, yes } => {
                    if numbers.is_empty() {
                        ensure!(yes, "deleting a pool requires --yes");
                        state.config.pools.remove(&name).context("unknown pool")?;
                    } else {
                        let pool = state.config.pools.get_mut(&name).context("unknown pool")?;
                        ensure!(
                            numbers.iter().all(|n| pool.numbers.contains(n)),
                            "one or more numbers are not in this pool"
                        );
                        pool.numbers.retain(|n| !numbers.contains(n));
                    }
                }
                PoolCommand::Limit { name, calls } => {
                    ensure!(calls > 0, "per-number limit must be positive");
                    for number in &state
                        .config
                        .pools
                        .get(&name)
                        .context("unknown pool")?
                        .numbers
                    {
                        state.config.caller_id_limits.insert(number.clone(), calls);
                    }
                }
                _ => unreachable!(),
            }
            Ok(())
        }),
    }
}

pub fn block(layout: &Layout, command: BlockCommand, json: bool) -> Result<()> {
    match command {
        BlockCommand::List => display(&layout.load()?.config.blocked_prefixes, json),
        command => edit(layout, json, |state| {
            match command {
                BlockCommand::Add { prefixes: values } => {
                    state.config.blocked_prefixes.extend(prefixes(values))
