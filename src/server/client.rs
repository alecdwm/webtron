use actix::Recipient;
use debug_stub_derive::DebugStub;
use uuid::Uuid;

use super::MessageOut;

#[derive(DebugStub)]
pub struct Client {
    pub id: Uuid,
    pub ip_address: Option<String>,
    #[debug_stub = "Recipient<MessageOut>"]
    pub address: Recipient<MessageOut>,
}

impl Client {
    pub fn new(id: Uuid, ip_address: Option<String>, address: Recipient<MessageOut>) -> Self {
        Self {
            id,
            ip_address,
            address,
        }
    }
}
