import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  loadPrograms,
  sealId,
  agentId,
  pda,
  enc,
  newMint,
  ata,
  mintToAta,
  tokenBalance,
  BN,
  Keypair,
  SystemProgram,
  TOKEN_PROGRAM_ID,
  PublicKey,
} from "./helpers";

const nowSec = () => Math.floor(Date.now() / 1000);

describe("magics-router", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const programs = loadPrograms(provider);
  const sv = programs.sealVault;
  const reg = programs.agentRegistry;
  const router = programs.router;
  const owner = provider.wallet.publicKey;
  const strategy = programs.passiveYield.programId;

  let mint: PublicKey;
  let ownerToken: PublicKey;
  let aid: Buffer;

  before(async () => {
    // Bind the vault once.
    const config = pda([enc("config")], sv.programId);
    try {
      await sv.methods
        .initialize(router.programId)
        .accountsPartial({ admin: owner, config, systemProgram: SystemProgram.programId })
        .rpc();
    } catch (_) {}

    mint = await newMint(provider, owner, 6);
    ownerToken = await ata(provider, mint, owner);
    await mintToAta(provider, mint, ownerToken, 1_000_000n);

    // Mint a seal and summon an agent over it.
    const params = {
      signer: Keypair.generate().publicKey,
      target: router.programId,
      selector: Buffer.alloc(8),
      valueCap: new BN(0),
      dailyCap: new BN(1_000_000),
      expiry: new BN(nowSec() + 3600),
      scopeHash: Buffer.alloc(32),
    };
    const sid = sealId(owner, params);
    const record = pda([enc("seal"), sid], sv.programId);
    await sv.methods
      .mintSeal([...sid], {
        signer: params.signer,
        target: params.target,
        selector: [...params.selector],
        valueCap: params.valueCap,
        dailyCap: params.dailyCap,
        expiry: params.expiry,
        scopeHash: [...params.scopeHash],
      })
      .accountsPartial({ owner, record, systemProgram: SystemProgram.programId })
      .rpc();

    const counterPda = pda([enc("counter"), owner.toBuffer()], reg.programId);
    let count = 0;
    try {
      count = (await reg.account.ownerCounter.fetch(counterPda)).count.toNumber();
    } catch (_) {}
    aid = agentId(owner, strategy, sid, new BN(count + 1));
    const agent = pda([enc("agent"), aid], reg.programId);
    await reg.methods
      .summon([...aid], [...sid], strategy, "router-test")
      .accountsPartial({
        owner,
        counter: counterPda,
        sealRecord: record,
        agent,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
  });

  const accs = () => {
    const agent = pda([enc("agent"), aid], reg.programId);
    const vaultToken = pda([enc("vault"), mint.toBuffer()], router.programId);
    const vaultAuthority = pda([enc("vault-auth")], router.programId);
    const balance = pda([enc("balance"), aid, mint.toBuffer()], router.programId);
    return { agent, vaultToken, vaultAuthority, balance };
  };

  it("deposits into the agent's balance", async () => {
    const { agent, vaultToken, vaultAuthority, balance } = accs();
    await router.methods
      .deposit([...aid], new BN(400_000))
      .accountsPartial({
        owner,
        agent,
        mint,
        ownerToken,
        vaultToken,
        vaultAuthority,
        balance,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    assert.equal((await router.account.balance.fetch(balance)).amount.toNumber(), 400_000);
    assert.equal(await tokenBalance(provider, vaultToken), 400_000n);
  });

  it("withdraws back out to a destination", async () => {
    const { agent, vaultToken, vaultAuthority, balance } = accs();
    await router.methods
      .withdraw([...aid], new BN(150_000))
      .accountsPartial({
        owner,
        agent,
        mint,
        vaultToken,
        vaultAuthority,
        destination: ownerToken,
        balance,
        tokenProgram: TOKEN_PROGRAM_ID,
      })
      .rpc();

    assert.equal((await router.account.balance.fetch(balance)).amount.toNumber(), 250_000);
    assert.equal(await tokenBalance(provider, ownerToken), 750_000n);
  });

  it("rejects an over-withdrawal", async () => {
    const { agent, vaultToken, vaultAuthority, balance } = accs();
    let failed = false;
    try {
      await router.methods
        .withdraw([...aid], new BN(10_000_000))
        .accountsPartial({
          owner,
          agent,
          mint,
          vaultToken,
          vaultAuthority,
          destination: ownerToken,
          balance,
          tokenProgram: TOKEN_PROGRAM_ID,
        })
        .rpc();
    } catch (_) {
      failed = true;
    }
    assert.isTrue(failed, "expected InsufficientBalance");
  });
});
