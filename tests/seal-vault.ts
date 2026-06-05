import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import { loadPrograms, sealId, pda, enc, BN, Keypair, SystemProgram } from "./helpers";

const nowSec = () => Math.floor(Date.now() / 1000);

describe("seal-vault", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const programs = loadPrograms(provider);
  const sv = programs.sealVault;
  const owner = provider.wallet.publicKey;
  const routerProgram = programs.router.programId;

  const makeParams = (overrides: any = {}) => ({
    signer: Keypair.generate().publicKey,
    target: routerProgram,
    selector: Buffer.alloc(8),
    valueCap: new BN(0),
    dailyCap: new BN(1_000_000),
    expiry: new BN(nowSec() + 3600),
    scopeHash: Buffer.alloc(32),
    ...overrides,
  });

  const mint = async (params: any) => {
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
    return { sid, record };
  };

  before(async () => {
    const config = pda([enc("config")], sv.programId);
    try {
      await sv.methods
        .initialize(routerProgram)
        .accountsPartial({ admin: owner, config, systemProgram: SystemProgram.programId })
        .rpc();
    } catch (_) {
      /* already bound */
    }
  });

  it("mints a seal under its derived id", async () => {
    const params = makeParams();
    const { record } = await mint(params);
    const rec: any = await sv.account.sealRecord.fetch(record);
    assert.equal(rec.owner.toBase58(), owner.toBase58());
    assert.equal(rec.revoked, false);
    assert.equal(rec.nonce.toNumber(), 0);
    assert.equal(rec.seal.signer.toBase58(), params.signer.toBase58());
  });

  it("revokes a seal with a reason", async () => {
    const { sid, record } = await mint(makeParams());
    await sv.methods
      .revoke([...sid], "compromised")
      .accountsPartial({ owner, record })
      .rpc();
    const rec: any = await sv.account.sealRecord.fetch(record);
    assert.equal(rec.revoked, true);
    assert.equal(rec.revokeReason, "compromised");
  });

  it("revoke_all halts a batch of live seals", async () => {
    const a = await mint(makeParams());
    const b = await mint(makeParams());
    await sv.methods
      .revokeAll()
      .accountsPartial({ owner })
      .remainingAccounts([
        { pubkey: a.record, isWritable: true, isSigner: false },
        { pubkey: b.record, isWritable: true, isSigner: false },
      ])
      .rpc();
    for (const x of [a, b]) {
      const rec: any = await sv.account.sealRecord.fetch(x.record);
      assert.equal(rec.revoked, true);
      assert.equal(rec.revokeReason, "revoke-all");
    }
  });

  it("rejects a seal id that doesn't match its params", async () => {
    const params = makeParams();
    const wrong = pda([enc("seal"), Buffer.alloc(32, 7)], sv.programId);
    let failed = false;
    try {
      await sv.methods
        .mintSeal(Array(32).fill(7), {
          signer: params.signer,
          target: params.target,
          selector: [...params.selector],
          valueCap: params.valueCap,
          dailyCap: params.dailyCap,
          expiry: params.expiry,
          scopeHash: [...params.scopeHash],
        })
        .accountsPartial({ owner, record: wrong, systemProgram: SystemProgram.programId })
        .rpc();
    } catch (_) {
      failed = true;
    }
    assert.isTrue(failed, "expected SealIdMismatch");
  });
});
