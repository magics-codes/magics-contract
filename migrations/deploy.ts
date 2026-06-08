// Post-deploy wiring. `anchor deploy` uploads the program binaries; this runs
// afterwards to bind the seal vault to the router — the one piece of cross-
// program state that can't be fixed at compile time, because the vault sits
// below the router in the dependency graph and only learns its id here.
//
//   anchor deploy --provider.cluster devnet
//   anchor migrate --provider.cluster devnet

const anchor = require("@coral-xyz/anchor");
const { PublicKey, SystemProgram } = require("@solana/web3.js");

module.exports = async function (provider: any) {
  anchor.setProvider(provider);

  const idl = (name: string) => require(`../target/idl/${name}.json`);
  const sealVault = new anchor.Program(idl("seal_vault"), provider);
  const router = new anchor.Program(idl("magics_router"), provider);

  const [config] = PublicKey.findProgramAddressSync(
    [Buffer.from("config")],
    sealVault.programId
  );

  try {
    await sealVault.methods
      .initialize(router.programId)
      .accountsPartial({
        admin: provider.wallet.publicKey,
        config,
        systemProgram: SystemProgram.programId,
      })
      .rpc();
    console.log("seal-vault bound to router", router.programId.toBase58());
  } catch (e: any) {
    console.log("seal-vault already bound, or:", e.message);
  }
};
