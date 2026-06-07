import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  Ed25519Program,
  ComputeBudgetProgram,
  SYSVAR_INSTRUCTIONS_PUBKEY,
  SYSVAR_RENT_PUBKEY,
} from "@solana/web3.js";
import {
  loadPrograms,
  sealId,
  agentId,
  castMessage,
  dataHash,
  pda,
  enc,
  newMint,
  ata,
  mintToAta,
  BN,
  Keypair,
  SystemProgram,
  TOKEN_PROGRAM_ID,
  PublicKey,
} from "./helpers";

const nowSec = () => Math.floor(Date.now() / 1000);

describe("passive-yield (full cast flow)", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const programs = loadPrograms(provider);
  const { sealVault: sv, agentRegistry: reg, router, mockYield: my, passiveYield: py } = programs;
  const owner = provider.wallet.publicKey;
  const strategy = py.programId;

  let mint: PublicKey;
  let ownerToken: PublicKey;
  let signer: Keypair; // the seal's ephemeral signer
  let aid: Buffer;
  let sid: Buffer;

  // PDA bundle reused across casts.
  let P: any;

  before(async () => {
    // 1) bind the vault to the router (idempotent across files).
    try {
      await sv.methods
        .initialize(router.programId)
        .accountsPartial({
          admin: owner,
          config: pda([enc("config")], sv.programId),
          systemProgram: SystemProgram.programId,
        })
        .rpc();
    } catch (_) {}

    // 2) asset mint + funded owner account.
    mint = await newMint(provider, owner, 6);
    ownerToken = await ata(provider, mint, owner);
    await mintToAta(provider, mint, ownerToken, 1_000_000n);

    // 3) yield vault for the asset.
    const mockVault = pda([enc("yvault"), mint.toBuffer()], my.programId);
    const shareMint = pda([enc("yshare"), mint.toBuffer()], my.programId);
    const mockUnderlying = pda([enc("yunder"), mint.toBuffer()], my.programId);
    const mockVaultAuthority = pda([enc("yauth"), mint.toBuffer()], my.programId);
    await my.methods
      .initVault(0)
      .accountsPartial({
        payer: owner,
        assetMint: mint,
        vault: mockVault,
        vaultAuthority: mockVaultAuthority,
        shareMint,
        underlyingVault: mockUnderlying,
        tokenProgram: TOKEN_PROGRAM_ID,
        systemProgram: SystemProgram.programId,
        rent: SYSVAR_RENT_PUBKEY,
      })
      .rpc();

    // 4) seal under an ephemeral signer, scoped to the router.
    signer = Keypair.generate();
    const params = {
      signer: signer.publicKey,
      target: router.programId,
      selector: Buffer.alloc(8),
      valueCap: new BN(0),
      dailyCap: new BN(1_000_000),
      expiry: new BN(nowSec() + 3600),
      scopeHash: Buffer.alloc(32),
    };
    sid = sealId(owner, params);
    const sealRecord = pda([enc("seal"), sid], sv.programId);
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
      .accountsPartial({ owner, record: sealRecord, systemProgram: SystemProgram.programId })
      .rpc();

    // 5) summon the agent.
    const counterPda = pda([enc("counter"), owner.toBuffer()], reg.programId);
    let count = 0;
    try {
      count = (await reg.account.ownerCounter.fetch(counterPda)).count.toNumber();
    } catch (_) {}
    aid = agentId(owner, strategy, sid, new BN(count + 1));
    const agent = pda([enc("agent"), aid], reg.programId);
    await reg.methods
      .summon([...aid], [...sid], strategy, "compounder")
      .accountsPartial({
        owner,
        counter: counterPda,
        sealRecord,
        agent,
        systemProgram: SystemProgram.programId,
      })
      .rpc();

    // 6) the strategy's own token accounts (owned by its PDA).
    const strategyAuthority = pda([enc("strategy")], py.programId);
    const strategyAssetToken = await ata(provider, mint, strategyAuthority, true);
    const strategyShareToken = await ata(provider, shareMint, strategyAuthority, true);

    // 7) fund the agent's router balance.
    const vaultToken = pda([enc("vault"), mint.toBuffer()], router.programId);
    const vaultAuthority = pda([enc("vault-auth")], router.programId);
    const balance = pda([enc("balance"), aid, mint.toBuffer()], router.programId);
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

    P = {
      agent,
      sealRecord,
      sealVaultConfig: pda([enc("config")], sv.programId),
      castAuthority: pda([enc("cast-authority")], router.programId),
      activeCast: pda([enc("active"), aid], router.programId),
      position: pda([enc("position"), aid], py.programId),
      strategyAuthority,
      strategyAssetToken,
      strategyShareToken,
      shareMint,
      balance,
      vaultToken,
      vaultAuthority,
      mockVault,
      mockUnderlying,
      mockVaultAuthority,
    };
  });

  // Drive one cast: prepend an ed25519 proof, raise the compute limit, forward
  // every account the strategy needs.
  const cast = async (op: number) => {
    const data = Buffer.from([op]);
    const dh = dataHash(data);
    const deadline = new BN(nowSec() + 600);
    const nonce = (await sv.account.sealRecord.fetch(P.sealRecord)).nonce as anchor.BN;

    const message = castMessage(sv.programId, aid, nonce, deadline, dh);
    const edIx = Ed25519Program.createInstructionWithPrivateKey({
      privateKey: signer.secretKey,
      message,
    });
    const cuIx = ComputeBudgetProgram.setComputeUnitLimit({ units: 800_000 });

    const remaining = [
      { pubkey: owner, isSigner: true, isWritable: true }, // payer
      { pubkey: P.position, isSigner: false, isWritable: true },
      { pubkey: P.strategyAuthority, isSigner: false, isWritable: false },
      { pubkey: mint, isSigner: false, isWritable: false },
      { pubkey: P.shareMint, isSigner: false, isWritable: true },
      { pubkey: P.strategyAssetToken, isSigner: false, isWritable: true },
      { pubkey: P.strategyShareToken, isSigner: false, isWritable: true },
      { pubkey: router.programId, isSigner: false, isWritable: false },
      { pubkey: P.activeCast, isSigner: false, isWritable: false },
      { pubkey: P.balance, isSigner: false, isWritable: true },
      { pubkey: P.vaultToken, isSigner: false, isWritable: true },
      { pubkey: P.vaultAuthority, isSigner: false, isWritable: false },
      { pubkey: my.programId, isSigner: false, isWritable: false },
      { pubkey: P.mockVault, isSigner: false, isWritable: false },
      { pubkey: P.mockUnderlying, isSigner: false, isWritable: true },
      { pubkey: P.mockVaultAuthority, isSigner: false, isWritable: false },
      { pubkey: TOKEN_PROGRAM_ID, isSigner: false, isWritable: false },
      { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
    ];

    await router.methods
      .cast([...aid], deadline, data)
      .accountsPartial({
        caster: owner,
        agent: P.agent,
        sealRecord: P.sealRecord,
        sealVaultConfig: P.sealVaultConfig,
        castAuthority: P.castAuthority,
        instructionsSysvar: SYSVAR_INSTRUCTIONS_PUBKEY,
        activeCast: P.activeCast,
        strategyProgram: strategy,
        sealVaultProgram: sv.programId,
        systemProgram: SystemProgram.programId,
      })
      .remainingAccounts(remaining)
      .preInstructions([edIx, cuIx])
      .rpc();
  };

  it("compounds idle balance into the yield vault", async () => {
    await cast(0x01);
    assert.equal((await router.account.balance.fetch(P.balance)).amount.toNumber(), 0);
    assert.equal((await py.account.position.fetch(P.position)).shares.toNumber(), 400_000);
  });

  it("harvests yield back into the agent's balance", async () => {
    // Accrue 10% yield and fund it in the vault.
    await my.methods
      .setYieldBps(1000)
      .accountsPartial({ vault: P.mockVault })
      .rpc();
    await mintToAta(provider, mint, P.mockUnderlying, 40_000n);

    await cast(0x02);
    assert.equal((await py.account.position.fetch(P.position)).shares.toNumber(), 0);
    assert.equal((await router.account.balance.fetch(P.balance)).amount.toNumber(), 440_000);
  });
});
