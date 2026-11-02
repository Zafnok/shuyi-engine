// Native-compatible Workshop JSON replay from a browser-target Wasm module.
// Usage: node replay.mjs <wasm-path> workshop <seed> <comma-actions|- >
//        node replay.mjs <wasm-path> --batch <cases.json|->
// Batch input is [{"game":"workshop","seed":"11","actions":["gather",...]}, ...].

import { readFile } from "node:fs/promises";
import { createFixture } from "./bridge.mjs";

async function stdinText() {
  const chunks = [];
  for await (const chunk of process.stdin) chunks.push(chunk);
  return Buffer.concat(chunks).toString("utf8");
}

async function main() {
  const [wasmPath, mode, seedOrPath, actions] = process.argv.slice(2);
  if (!wasmPath) throw new Error("Wasm path required");
  const fixture = await createFixture(new Uint8Array(await readFile(wasmPath)));
  if (mode === "workshop") {
    if (seedOrPath === undefined || actions === undefined) {
      throw new Error("Usage: replay.mjs <wasm> workshop <seed> <comma-actions|->");
    }
    process.stdout.write(`${fixture.replay(seedOrPath, actions)}\n`);
    return;
  }
  if (mode === "--batch") {
    if (!seedOrPath) throw new Error("Batch JSON file or - required");
    const input = seedOrPath === "-" ? await stdinText() : await readFile(seedOrPath, "utf8");
    const cases = JSON.parse(input);
    if (!Array.isArray(cases)) throw new Error("Batch input must be an array");
    const results = cases.map((entry) => {
      if (entry.game !== "workshop") throw new Error(`Unsupported game: ${entry.game}`);
      return fixture.replay(entry.seed, entry.actions);
    });
    // Join the raw Rust JSON records, preserving exact u64 seeds in the output.
    process.stdout.write(`[${results.join(",")}]\n`);
    return;
  }
  throw new Error("Usage: replay.mjs <wasm> workshop <seed> <comma-actions|-> | --batch <cases.json|->");
}

main().catch((error) => {
  process.stderr.write(`${error.message}\n`);
  process.exitCode = 1;
});
