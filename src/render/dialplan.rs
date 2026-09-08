use crate::model::State;
use std::fmt::Write;

pub fn render(state: &State) -> String {
    let mut output = String::from(
        "[general]\nstatic=yes\nwriteprotect=yes\nclearglobalvars=yes\n\n[default]\nexten => _.,1,Hangup(21)\n\n[voipctl-inbound]\nexten => _.,1,Hangup(21)\nexten => s,1,Hangup(21)\n\n",
    );
    render_number_limit(state, &mut output);
    render_pools(state, &mut output);
    for (name, customer) in &state.config.customers {
        for (number, extension) in &customer.extensions {
            writeln!(
                output,
                "[voipctl-ext-{number}]\nexten => _[+0-9].,1,Set(RAW=${{EXTEN}})"
            )
            .unwrap();
            line(&mut output, "Set(CDR(customer_ext)=${VOIP_EXT})");
            line(
                &mut output,
                "Set(CDR(src_ip)=${CHANNEL(pjsip,remote_addr)})",
            );
            line(&mut output, &format!("Set(CDR(accountcode)={name})"));
            line(
                &mut output,
                "GotoIf($[${REGEX(\"^[+0-9]+$\" ${RAW})} = 0]?invalid)",
            );
            line(
                &mut output,
                &format!("Set(CID_SPEC={})", extension.caller_id),
            );
            line(&mut output, "GotoIf($[\"${RAW:0:2}\" = \"11\"]?alternate)");
            line(
                &mut output,
                "GotoIf($[\"${RAW:0:2}\" = \"10\"]?strip:normalize)",
            );
            if let Some(cid) = &extension.alternate_caller_id {
                labeled(&mut output, "alternate", &format!("Set(CID_SPEC={cid})"));
                line(&mut output, "Goto(strip)");
            } else {
                labeled(&mut output, "alternate", "Goto(nocid)");
            }
            labeled(&mut output, "strip", "Set(RAW=${RAW:2})");
            labeled(
                &mut output,
                "normalize",
                "GotoIf($[${REGEX(\"^([+][1-9][0-9]*|[0-9]+)$\" ${RAW})} = 0]?invalid)",
            );
            line(&mut output, "Set(NUM=${RAW})");
            line(
                &mut output,
                "ExecIf($[\"${RAW:0:1}\" = \"+\"]?Set(NUM=${RAW:1}))",
            );
            line(
                &mut output,
                "ExecIf($[\"${RAW:0:2}\" = \"00\"]?Set(NUM=${RAW:2}))",
            );
            if let Some(country) = &customer.default_country {
                line(
                    &mut output,
                    &format!(
                        "ExecIf($[\"${{RAW:0:1}}\" = \"0\" & \"${{RAW:0:2}}\" != \"00\"]?Set(NUM={country}${{RAW:1}}))"
                    ),
                );
            }
            line(
                &mut output,
                "GotoIf($[${REGEX(\"^[1-9][0-9]{6,14}$\" ${NUM})} = 0]?invalid)",
            );
            line(&mut output, "Set(DEST=+${NUM})");
            line(&mut output, "Set(CDR(dialed)=${DEST})");
            for prefix in &state.config.blocked_prefixes {
                line(
                    &mut output,
                    &format!(
                        "GotoIf($[\"${{NUM:0:{}}}\" = \"{prefix}\"]?denied)",
                        prefix.len()
                    ),
                );
            }
            for prefix in &customer.allowed_prefixes {
                line(
                    &mut output,
                    &format!(
                        "GotoIf($[\"${{NUM:0:{}}}\" = \"{prefix}\"]?allocate)",
                        prefix.len()
                    ),
                );
            }
            line(&mut output, "Goto(denied)");
            labeled(
                &mut output,
                "allocate",
                "Set(LOCKED=${LOCK(voipctl-allocation)})",
            );
            line(&mut output, "GotoIf($[${LOCKED} != 1]?limit)");
            line(
                &mut output,
                &format!(
                    "GotoIf($[${{GROUP_COUNT({name}@voipctl-customer)}} >= {}]?full)",
                    customer.max_calls
                ),
            );
            line(
                &mut output,
                &format!(
                    "GotoIf($[${{GROUP_COUNT({}@voipctl-trunk)}} >= {}]?full)",
                    customer.trunk, state.config.trunks[&customer.trunk].max_calls
                ),
            );
            line(
                &mut output,
                "GotoIf($[\"${CID_SPEC:0:5}\" = \"pool:\"]?pool)",
            );
            line(&mut output, "Set(CID=${CID_SPEC})");
            line(&mut output, "Gosub(voipctl-cid-check,s,1)");
            line(&mut output, "Goto(selected)");
            labeled(&mut output, "pool", "Gosub(voipctl-pool-${CID_SPEC:5},s,1)");
            labeled(&mut output, "selected", "GotoIf($[${AVAILABLE} != 1]?full)");
            line(&mut output, &format!("Set(GROUP(voipctl-customer)={name})"));
            line(
                &mut output,
                &format!("Set(GROUP(voipctl-trunk)={})", customer.trunk),
            );
            line(&mut output, "Set(GROUP(voipctl-cid)=${CID:1})");
            line(&mut output, "Set(UNLOCKED=${UNLOCK(voipctl-allocation)})");
            line(&mut output, "Set(CALLERID(name)=)");
            line(&mut output, "Set(CALLERID(num)=${CID})");
            line(
                &mut output,
                &format!("Set(TIMEOUT(absolute)={})", customer.max_call_seconds),
            );
            line(
                &mut output,
                &format!(
                    "Dial(PJSIP/${{DEST}}@trunk-{},{})",
                    customer.trunk, state.config.server.dial_timeout
                ),
            );
            line(&mut output, "Set(CDR_PROP(disable)=1)");
            line(&mut output, "Hangup()");
            labeled(
                &mut output,
                "full",
                "Set(UNLOCKED=${UNLOCK(voipctl-allocation)})",
            );
            line(&mut output, "Goto(limit)");
            for (label, cause, reason) in [
                ("invalid", 28, "invalid-number"),
                ("denied", 21, "destination-not-allowed"),
                ("limit", 34, "concurrent-limit"),
                ("nocid", 21, "no-caller-id"),
            ] {
                labeled(
                    &mut output,
                    label,
                    &format!("Set(CDR(userfield)=REJECTED:{reason})"),
                );
                line(&mut output, &format!("Hangup({cause})"));
            }
            output.push('\n');
        }
    }
    output
}

fn render_number_limit(state: &State, output: &mut String) {
    writeln!(
        output,
        "[voipctl-cid-check]\nexten => s,1,Set(CID_LIMIT={})",
        state.config.server.max_calls_per_number
    )
    .unwrap();
    for (number, limit) in &state.config.caller_id_limits {
        line(
            output,
            &format!("ExecIf($[\"${{CID}}\" = \"{number}\"]?Set(CID_LIMIT={limit}))"),
        );
    }
    line(output, "Set(AVAILABLE=1)");
    line(
        output,
        "ExecIf($[${CID_LIMIT} > 0 & ${GROUP_COUNT(${CID:1}@voipctl-cid)} >= ${CID_LIMIT}]?Set(AVAILABLE=0))",
    );
    line(output, "Return()");
    output.push('\n');
}

fn render_pools(state: &State, output: &mut String) {
    for (name, pool) in &state.config.pools {
        let count = pool.numbers.len();
        writeln!(
            output,
            "[voipctl-pool-{name}]\nexten => s,1,Set(INDEX=${{RAND(0,{})}})",
            count - 1
        )
        .unwrap();
        line(output, "Set(ATTEMPTS=0)");
        labeled(output, "pick", "Goto(candidate-${INDEX})");
        for (index, number) in pool.numbers.iter().enumerate() {
            labeled(
                output,
                &format!("candidate-{index}"),
                &format!("Set(CID={number})"),
            );
            line(output, "Goto(check)");
        }
        labeled(output, "check", "Gosub(voipctl-cid-check,s,1)");
        line(output, "GotoIf($[${AVAILABLE} = 1]?done)");
        line(output, "Set(ATTEMPTS=$[${ATTEMPTS} + 1])");
        line(output, &format!("GotoIf($[${{ATTEMPTS}} >= {count}]?done)"));
        line(output, &format!("Set(INDEX=$[(${{INDEX}} + 1) % {count}])"));
        line(output, "Goto(pick)");
        labeled(output, "done", "Return()");
        output.push('\n');
    }
}

fn line(output: &mut String, value: &str) {
    writeln!(output, " same => n,{value}").unwrap();
}
fn labeled(output: &mut String, label: &str, value: &str) {
    writeln!(output, " same => n({label}),{value}").unwrap();
}
