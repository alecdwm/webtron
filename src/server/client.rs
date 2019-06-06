use crate::game::Player;
use crate::server::MessageOut;
use actix::Recipient;
use debug_stub_derive::DebugStub;
use uuid::Uuid;

#[derive(DebugStub)]
pub struct Client {
    id: Uuid,
    #[debug_stub = "Recipient<MessageOut>"]
    addr: Recipient<MessageOut>,
    player: Option<Player>,
}

impl Client {
    pub fn new(id: Uuid, addr: Recipient<MessageOut>) -> Self {
        Self {
            id,
            addr,
            player: None,
        }
    }

    pub fn id(&self) -> &Uuid {
        &self.id
    }

    pub fn addr(&self) -> &Recipient<MessageOut> {
        &self.addr
    }

    pub fn configure_player(&mut self, player: Player) {
        self.player = Some(player);
    }
}
