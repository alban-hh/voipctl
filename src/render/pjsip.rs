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
