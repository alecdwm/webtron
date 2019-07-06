use crate::server::MessageOut;
use actix::Recipient;
use debug_stub_derive::DebugStub;
use uuid::Uuid;

#[derive(DebugStub)]
pub struct Client {
    id: Uuid,
    ip_address: Option<String>,
    #[debug_stub = "Recipient<MessageOut>"]
    addr: Recipient<MessageOut>,
}

impl Client {
    pub fn new(id: Uuid, ip_address: Option<String>, addr: Recipient<MessageOut>) -> Self {
        Self {
            id,
            ip_address,
            addr,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn ip_address(&self) -> &Option<String> {
        &self.ip_address
    }

    pub fn addr(&self) -> &Recipient<MessageOut> {
        &self.addr
    }
}
