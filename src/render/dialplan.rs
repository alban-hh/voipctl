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
