// Write the program-id manifest the CLI and web app read at start-up — the
// Solana counterpart of the EVM `deployments/{chainId}.json`. Program ids are
// the deploy keypairs' public keys, identical on every cluster, so this just
// stamps which cluster they were last pushed to and when.
//
//   npx ts-node scripts/record-deployment.ts devnet

import * as fs from "fs";
import * as path from "path";

const PROGRAMS: Record<string, string> = {
  sealVault: "Hs1WNEErp5rqhy8JvofPzL8UYHFaWj9gRDjofhNqQvhZ",
  agentRegistry: "31SxC6ivUkHdcUnvR23wqGyJgdmiHWR7UWZuWW42cYCR",
  magicsRouter: "EyZ4qCWwXZwh9nWnZo1nvNoThsY96yprV2APWEEChCBy",
  passiveYield: "Gd39nuaCTLMv5J6Q2jA8khmtEKwvsdBafTWK8twLvR5q",
  mockYield: "Cj2KQGwVxDYgzXk4q8XMNQBCsCNxo3dRkWtG6eng66gN",
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
