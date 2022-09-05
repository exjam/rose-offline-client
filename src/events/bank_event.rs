use rose_game_common::messages::ClientEntityId;

pub enum BankEvent {
    OpenBankFromClientEntity { client_entity_id: ClientEntityId },
    Show,
}
