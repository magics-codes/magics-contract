// Write the program-id manifest the CLI and web app read at start-up — the
// Solana counterpart of the EVM `deployments/{chainId}.json`. Program ids are
// the deploy keypairs' public keys, identical on every cluster, so this just
// stamps which cluster they were last pushed to and when.
//
//   npx ts-node scripts/record-deployment.ts devnet

import * as fs from "fs";
import * as path from "path";

const PROGRAMS: Record<string, string> = {
  sealVault: "Chydhx9FZ7dYwvNoRzbE8VMNHiQCJ8xWiydRuSoY9Q7W",
  agentRegistry: "9qEVeDELh9wSRfKFcWohWkYjW2MfoKXepFTXeqN9TFsN",
  magicsRouter: "2c2FBbgCpB2VPhqMDsTX6VXJEhcnczSSTw5eR3DZrpUu",
  passiveYield: "HPEdHcgxJVQX2VRcYM2fMXn963sXXmwhuxC9SwWLQD6n",
  mockYield: "4339iYHiD6y52hcCWrUdN7tjH3rSEwnmyKdQZfFpngWC",
};

const cluster = process.argv[2] || "devnet";
const out = {
  cluster,
  recordedAt: Math.floor(Date.now() / 1000),
  programs: PROGRAMS,
};

const dir = path.join(__dirname, "..", "deployments");
fs.mkdirSync(dir, { recursive: true });
const file = path.join(dir, `${cluster}.json`);
fs.writeFileSync(file, JSON.stringify(out, null, 2) + "\n");
console.log(`wrote ${file}`);
