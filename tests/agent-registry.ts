import * as anchor from "@coral-xyz/anchor";
import { assert } from "chai";
import {
  loadPrograms,
  sealId,
  agentId,
  pda,
  enc,
  BN,
  Keypair,
  SystemProgram,
} from "./helpers";

const nowSec = () => Math.floor(Date.now() / 1000);

describe("agent-registry", () => {
  const provider = anchor.AnchorProvider.env();
  anchor.setProvider(provider);
  const programs = loadPrograms(provider);
  const sv = programs.sealVault;
  const reg = programs.agentRegistry;
  const owner = provider.wallet.publicKey;
  const strategy = programs.passiveYield.programId;

  // Mint a fresh seal and return its id + record PDA.
  const freshSeal = async () => {
    const params = {
      signer: Keypair.generate().publicKey,
      target: programs.router.programId,
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
    return { sid, record };
  };

  const summon = async (name: string) => {
    const { sid, record } = await freshSeal();
    const counterPda = pda([enc("counter"), owner.toBuffer()], reg.programId);
    let count = 0;
    try {
      count = (await reg.account.ownerCounter.fetch(counterPda)).count.toNumber();
    } catch (_) {
      /* first summon */
    }
    const aid = agentId(owner, strategy, sid, new BN(count + 1));
    const agent = pda([enc("agent"), aid], reg.programId);
    await reg.methods
      .summon([...aid], [...sid], strategy, name)
      .accountsPartial({
        owner,
        counter: counterPda,
        sealRecord: record,
        agent,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    return { aid, agent, sid };
  };

  it("summons an agent under a derived id", async () => {
    const { agent } = await summon("sol-yield-bot");
    const a: any = await reg.account.agent.fetch(agent);
    assert.equal(a.owner.toBase58(), owner.toBase58());
    assert.equal(a.strategy.toBase58(), strategy.toBase58());
    assert.equal(a.name, "sol-yield-bot");
    assert.deepEqual(a.status, { active: {} });
  });

  it("walks the status lifecycle and blocks moves on a halted agent", async () => {
    const { aid, agent } = await summon("lifecycle");
    await reg.methods.pause([...aid], "cooling off").accountsPartial({ owner, agent }).rpc();
    assert.deepEqual((await reg.account.agent.fetch(agent)).status, { paused: {} });

    await reg.methods.resume([...aid]).accountsPartial({ owner, agent }).rpc();
    assert.deepEqual((await reg.account.agent.fetch(agent)).status, { active: {} });

    await reg.methods.halt([...aid], "done").accountsPartial({ owner, agent }).rpc();
    assert.deepEqual((await reg.account.agent.fetch(agent)).status, { halted: {} });

    let failed = false;
    try {
      await reg.methods.resume([...aid]).accountsPartial({ owner, agent }).rpc();
    } catch (_) {
      failed = true;
    }
    assert.isTrue(failed, "a halted agent cannot resume");
  });

  it("rotates the seal under a live agent", async () => {
    const { aid, agent } = await summon("rotate");
    const next = await freshSeal();
    await reg.methods
      .rotateSeal([...aid], [...next.sid])
      .accountsPartial({ owner, agent, newSealRecord: next.record })
      .rpc();
    const a: any = await reg.account.agent.fetch(agent);
    assert.deepEqual([...a.seal], [...next.sid]);
  });
});
