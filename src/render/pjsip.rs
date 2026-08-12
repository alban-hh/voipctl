use crate::model::State;
use std::fmt::Write;

pub fn render(state: &State) -> String {
    let mut output = String::from(
        "[global]\ntype=global\nuser_agent=voipctl\nendpoint_identifier_order=ip,username,auth_username\n\n[transport-udp]\ntype=transport\nprotocol=udp\n",
    );
    let server = &state.config.server;
    writeln!(output, "bind={}:{}\n", server.bind_address, server.sip_port).unwrap();
    for (id, trunk) in &state.config.trunks {
        let Some(credentials) = state.secrets.trunks.get(id) else {
            continue;
        };
        let name = format!("trunk-{id}");
        writeln!(output, "[{name}]\ntype=endpoint\ntransport=transport-udp\ncontext=voipctl-inbound\ndisallow=all\nallow=ulaw,alaw\noutbound_auth={name}-auth\naors={name}\nfrom_domain={}\ndirect_media=no\ndtmf_mode=rfc4733\nallow_transfer=no\nallow_subscribe=no\ntrust_id_inbound=no\nsend_pai=no\nsend_rpid=no\n", trunk.host).unwrap();
        writeln!(
            output,
            "[{name}-auth]\ntype=auth\nauth_type=userpass\nusername={}\npassword={}\n",
            credentials.username, credentials.password
        )
        .unwrap();
        writeln!(
            output,
            "[{name}]\ntype=aor\ncontact=sip:{}:{}\nqualify_frequency=60\n",
            trunk.host, trunk.port
        )
        .unwrap();
        if !trunk.signaling.is_empty() {
            writeln!(output, "[{name}-identify]\ntype=identify\nendpoint={name}").unwrap();
            for network in &trunk.signaling {
                writeln!(output, "match={network}").unwrap();
            }
            output.push('\n');
        }
    }
    for (name, customer) in &state.config.customers {
        for number in customer.extensions.keys() {
            writeln!(output, "[{number}]\ntype=endpoint\ntransport=transport-udp\ncontext=voipctl-ext-{number}\naccountcode={name}\ndisallow=all\nallow=ulaw,alaw\nauth={number}\naors={number}\nidentify_by=username,auth_username\ndirect_media=no\nrtp_symmetric=yes\nforce_rport=yes\nrewrite_contact=yes\ndtmf_mode=rfc4733\nallow_transfer=no\nallow_subscribe=no\ntrust_id_inbound=no\nsend_pai=no\nsend_rpid=no\nice_support=no\nset_var=VOIP_EXT={number}").unwrap();
            if !customer.source_ips.is_empty() {
                output.push_str("deny=0.0.0.0/0\n");
                for network in &customer.source_ips {
                    writeln!(output, "permit={network}").unwrap();
                }
            }
            writeln!(output, "\n[{number}]\ntype=auth\nauth_type=userpass\nusername={number}\npassword={}\n\n[{number}]\ntype=aor\nmax_contacts=1\nremove_existing=yes\nminimum_expiration=60\ndefault_expiration=300\nmaximum_expiration=3600\n", state.secrets.extensions[number]).unwrap();
        }
    }
    output
}
