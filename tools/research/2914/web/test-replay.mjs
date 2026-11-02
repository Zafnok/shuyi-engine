// Compare complete native and browser-target Wasm replay records.
// Usage: node test-replay.mjs <wasm-path> <native-fixture-path>

import assert from "node:assert/strict";
import { spawnSync } from "node:child_process";
import { readFile } from "node:fs/promises";
import { createFixture } from "./bridge.mjs";

const [wasmPath, nativePath] = process.argv.slice(2);
if (!wasmPath || !nativePath) {
  throw new Error("Usage: test-replay.mjs <wasm-path> <native-fixture-path>");
}

const fixture = await createFixture(new Uint8Array(await readFile(wasmPath)));
const win = [
  "gather", "gather", "refine", "craft", "deliver", "gather",
  "rest", "gather", "refine", "craft", "deliver", "invalid", "rest",
];
const cases = [];
for (const seed of ["0", "1", "11", "55", "4294967295", "18446744073709551615"]) {
  cases.push([seed, []]);
  cases.push([seed, win]);
  cases.push([seed, [...Array(16).fill("rest"), "gather"]]);
  cases.push([seed, ["craft", "refine", "deliver", "gather", "gather", "gather", "gather", "rest", "state"]]);
}

let random = 0x2914;
const actions = ["gather", "refine", "craft", "deliver", "rest", "invalid", "state"];
for (let index = 0; index < 80; index += 1) {
  random = (Math.imul(random, 1664525) + 1013904223) >>> 0;
  const seed = String(random);
  const trace = [];
  for (let step = 0; step < 20; step += 1) {
    random = (Math.imul(random, 1664525) + 1013904223) >>> 0;
    trace.push(actions[random % actions.length]);
  }
  cases.push([seed, trace]);
}

for (const [seed, trace] of cases) {
  const native = spawnSync(nativePath, ["workshop", seed, trace.length ? trace.join(",") : "-"], {
    encoding: "utf8",
    timeout: 30_000,
  });
  if (native.error) throw native.error;
  assert.equal(native.status, 0, native.stderr);
  assert.equal(fixture.replay(seed, trace), native.stdout.trim(), `seed ${seed}, ${trace.join(",")}`);
}
process.stdout.write(`Browser-target Wasm matched ${cases.length} complete native Workshop traces.\n`);
