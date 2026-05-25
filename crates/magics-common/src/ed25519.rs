use anchor_lang::prelude::*;
use anchor_lang::solana_program::ed25519_program::ID as ED25519_ID;
use anchor_lang::solana_program::sysvar::instructions::{
    load_current_index_checked, load_instruction_at_checked,
};

use crate::error::MagicsError;

/// The Ed25519 native program lays out a self-contained verify instruction as:
///   [0]      num_signatures (u8)
///   [1]      padding (u8)
///   [2..16]  one 14-byte offsets block (we only ever check the first)
/// followed by the public key, signature, and message at the offsets named in
/// that block. Field widths below index into the offsets block.
const NUM_SIGNATURES: usize = 0;
const OFFSETS_START: usize = 2;
const SIG_INSTR_IDX: usize = OFFSETS_START + 2;
const PK_OFFSET: usize = OFFSETS_START + 4;
const PK_INSTR_IDX: usize = OFFSETS_START + 6;
const MSG_OFFSET: usize = OFFSETS_START + 8;
const MSG_SIZE: usize = OFFSETS_START + 10;
const MSG_INSTR_IDX: usize = OFFSETS_START + 12;
const HEADER_END: usize = OFFSETS_START + 14;

const SELF_REF: u16 = u16::MAX;

/// Confirm the transaction carries an Ed25519 verification instruction proving
/// that `expected_signer` signed `expected_message`.
///
/// We don't re-check the signature math: the Ed25519 native program runs before
/// this instruction and aborts the whole transaction on a bad signature, so if
/// we're executing at all, every Ed25519 instruction in the tx has already been
/// verified by the runtime. Our job is only to confirm one of them carries the
/// pubkey and message we expect — the Solana analog of comparing `ECDSA.recover`
/// against `seal.signer`.
pub fn verify_cast_signature(
    ix_sysvar: &AccountInfo,
    expected_signer: &Pubkey,
    expected_message: &[u8],
) -> Result<()> {
    // The verify instruction must sit before us; scan everything earlier.
    let current = load_current_index_checked(ix_sysvar)? as usize;
    for i in 0..current {
        let ix = load_instruction_at_checked(i, ix_sysvar)?;
        if ix.program_id != ED25519_ID {
            continue;
        }
        if matches_signer_and_message(&ix.data, i as u16, expected_signer, expected_message)? {
            return Ok(());
        }
    }
    err!(MagicsError::MissingEd25519Instruction)
}

/// Pull the pubkey and message out of one Ed25519 instruction's data and
/// compare them. Returns Ok(false) when the layout points at a different
/// instruction (not our self-contained convention) so the scan can keep going.
fn matches_signer_and_message(
    data: &[u8],
    this_index: u16,
    expected_signer: &Pubkey,
    expected_message: &[u8],
) -> Result<bool> {
    if data.len() < HEADER_END {
        return err!(MagicsError::MalformedEd25519Instruction);
    }
    if data[NUM_SIGNATURES] != 1 {
        // We only accept a single-signature verify instruction.
        return Ok(false);
    }

    let read_u16 = |at: usize| u16::from_le_bytes([data[at], data[at + 1]]);
    let references_self = |idx: u16| idx == SELF_REF || idx == this_index;

    if !references_self(read_u16(SIG_INSTR_IDX))
        || !references_self(read_u16(PK_INSTR_IDX))
        || !references_self(read_u16(MSG_INSTR_IDX))
    {
        // Pubkey / message live in some other instruction's data — not the
        // self-contained shape we build off-chain. Skip it.
        return Ok(false);
    }

    let pk_offset = read_u16(PK_OFFSET) as usize;
    let msg_offset = read_u16(MSG_OFFSET) as usize;
    let msg_size = read_u16(MSG_SIZE) as usize;

    let pk_end = pk_offset
        .checked_add(32)
        .ok_or(MagicsError::MalformedEd25519Instruction)?;
    let msg_end = msg_offset
        .checked_add(msg_size)
        .ok_or(MagicsError::MalformedEd25519Instruction)?;
    if pk_end > data.len() || msg_end > data.len() {
        return err!(MagicsError::MalformedEd25519Instruction);
    }

    let signer_matches = &data[pk_offset..pk_end] == expected_signer.as_ref();
    let message_matches = &data[msg_offset..msg_end] == expected_message;
    Ok(signer_matches && message_matches)
}
