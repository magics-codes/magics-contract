import * as anchor from "@coral-xyz/anchor";
import {
  PublicKey,
  Keypair,
  SystemProgram,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  TOKEN_PROGRAM_ID,
  createMint,
  getOrCreateAssociatedTokenAccount,
  getAssociatedTokenAddressSync,
  createAssociatedTokenAccountInstruction,
  mintTo,
  getAccount,
} from "@solana/spl-token";
import { keccak256 } from "js-sha3";
import BN from "bn.js";

// ── byte helpers ────────────────────────────────────────────────────────────

export const enc = (s: string) => Buffer.from(s, "utf8");
export const le8 = (n: number | bigint | BN) =>
  new BN(n.toString()).toArrayLike(Buffer, "le", 8);
export const keccak = (...parts: Buffer[]) =>
  Buffer.from(keccak256.array(Buffer.concat(parts)));

// ── deterministic ids (must mirror the Rust derivations) ────────────────────

export interface SealParams {
  signer: PublicKey;
  target: PublicKey;
  selector: Buffer; // 8 bytes
  valueCap: BN;
  dailyCap: BN;
  expiry: BN;
  scopeHash: Buffer; // 32 bytes
}

export function sealId(owner: PublicKey, s: SealParams): Buffer {
  return keccak(
    enc("magics:seal:v1"),
    owner.toBuffer(),
    s.signer.toBuffer(),
    s.target.toBuffer(),
    s.selector,
    le8(s.valueCap),
    le8(s.dailyCap),
    le8(s.expiry),
    s.scopeHash
  );
}

export function agentId(
  owner: PublicKey,
  strategy: PublicKey,
  seal: Buffer,
  counter: BN
): Buffer {
  return keccak(
    enc("magics:agent:v1"),
    owner.toBuffer(),
    strategy.toBuffer(),
    seal,
    le8(counter)
  );
}

export function castMessage(
  vaultProgram: PublicKey,
  agent: Buffer,
  nonce: BN,
  deadline: BN,
  dh: Buffer
): Buffer {
  return Buffer.concat([
    enc("magics:cast:v1"),
    enc("magics"),
    enc("1"),
    vaultProgram.toBuffer(),
    agent,
    le8(nonce),
    le8(deadline),
    dh,
  ]);
}

export const dataHash = (data: Buffer) => keccak(data);

// ── PDA helper ──────────────────────────────────────────────────────────────

export const pda = (seeds: Buffer[], programId: PublicKey) =>
  PublicKey.findProgramAddressSync(seeds, programId)[0];

// ── program handles, loaded from the built IDLs ─────────────────────────────

// Loosely typed on purpose: tests address account namespaces and instruction
// builders by name, which anchor resolves at runtime from each IDL.
export interface Programs {
  sealVault: any;
  agentRegistry: any;
  router: any;
  mockYield: any;
  passiveYield: any;
}

export function loadPrograms(provider: anchor.AnchorProvider): Programs {
  const idl = (name: string) =>
    require(`../target/idl/${name}.json`) as anchor.Idl;
  return {
    sealVault: new anchor.Program(idl("seal_vault"), provider),
    agentRegistry: new anchor.Program(idl("agent_registry"), provider),
    router: new anchor.Program(idl("magics_router"), provider),
    mockYield: new anchor.Program(idl("mock_yield"), provider),
    passiveYield: new anchor.Program(idl("passive_yield"), provider),
  };
}

// ── SPL token helpers ───────────────────────────────────────────────────────

export async function newMint(
  provider: anchor.AnchorProvider,
  authority: PublicKey,
  decimals = 6
): Promise<PublicKey> {
  return createMint(
    provider.connection,
    (provider.wallet as anchor.Wallet).payer,
    authority,
    null,
    decimals
  );
}

export async function ata(
  provider: anchor.AnchorProvider,
  mint: PublicKey,
  owner: PublicKey,
  allowOwnerOffCurve = false
): Promise<PublicKey> {
  const address = getAssociatedTokenAddressSync(mint, owner, allowOwnerOffCurve);
  const ix = createAssociatedTokenAccountInstruction(
    provider.wallet.publicKey,
    address,
    owner,
    mint
  );
  const tx = new anchor.web3.Transaction().add(ix);
  try {
    await provider.sendAndConfirm(tx, []);
  } catch (_) {
    // already exists
  }
  return address;
}

export async function mintToAta(
  provider: anchor.AnchorProvider,
  mint: PublicKey,
  dest: PublicKey,
  amount: number | bigint
): Promise<void> {
  await mintTo(
    provider.connection,
    (provider.wallet as anchor.Wallet).payer,
    mint,
    dest,
    provider.wallet.publicKey,
    amount
  );
}

export async function tokenBalance(
  provider: anchor.AnchorProvider,
  account: PublicKey
): Promise<bigint> {
  return (await getAccount(provider.connection, account)).amount;
}

export const airdrop = async (
  provider: anchor.AnchorProvider,
  to: PublicKey,
  sol = 2
) => {
  const sig = await provider.connection.requestAirdrop(
    to,
    sol * anchor.web3.LAMPORTS_PER_SOL
  );
  await provider.connection.confirmTransaction(sig);
};

export { BN, PublicKey, Keypair, SystemProgram, SYSVAR_RENT_PUBKEY, TOKEN_PROGRAM_ID };
