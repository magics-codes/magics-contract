use anchor_lang::prelude::*;

use crate::state::Status;

#[event]
pub struct AgentSummoned {
    pub owner: Pubkey,
    pub agent_id: [u8; 32],
    pub strategy: Pubkey,
    pub seal: [u8; 32],
    pub name: String,
}

#[event]
pub struct AgentStatusChanged {
    pub agent_id: [u8; 32],
    pub from: Status,
    pub to: Status,
    pub reason: String,
}

#[event]
pub struct AgentSealRotated {
    pub agent_id: [u8; 32],
    pub old_seal: [u8; 32],
    pub new_seal: [u8; 32],
}
