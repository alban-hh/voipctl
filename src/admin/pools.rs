use super::{display, edit};
use crate::{cli::PoolCommand, model::Pool, storage::Layout};
use anyhow::{Context, Result, ensure};
use std::fs;

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
