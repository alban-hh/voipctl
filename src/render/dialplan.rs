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
