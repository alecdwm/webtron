use actix::Recipient;
use debug_stub_derive::DebugStub;
use uuid::Uuid;

use super::MessageOut;

#[derive(DebugStub)]
pub struct Client {
    id: Uuid,
    ip_address: Option<String>,
    #[debug_stub = "Recipient<MessageOut>"]
    address: Recipient<MessageOut>,
}

impl Client {
    pub fn new(id: Uuid, ip_address: Option<String>, address: Recipient<MessageOut>) -> Self {
        Self {
            id,
            ip_address,
            address,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn ip_address(&self) -> &Option<String> {
        &self.ip_address
    }

    pub fn address(&self) -> &Recipient<MessageOut> {
        &self.address
    }
}
