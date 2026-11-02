// Inspect and replay the ticket 2914 browser fixture with Node's V8 debugger.
// Usage: node inspector-trace.mjs <fixture.wasm> <u64-seed> <comma-actions|->
//        [--disasm-out <path>]

import { createHash } from 'node:crypto';
import { readFile, writeFile } from 'node:fs/promises';
import inspector from 'node:inspector';

function progress(stage) {
  if (process.env.SHUYI_INSPECT_DEBUG) console.error(`inspector: ${stage}`);
}

function post(session, method, params = {}) {
  return new Promise((resolve, reject) => {
    session.post(method, params, (error, answer) => {
      if (error) reject(error);
      else resolve(answer);
    });
  });
}

async function disassemble(session, scriptId, full) {
  const first = await post(session, 'Debugger.disassembleWasmModule', { scriptId });
  const lines = [...first.chunk.lines];
  if (full && first.streamId) {
    while (true) {
      const next = await post(session, 'Debugger.nextWasmDisassemblyChunk', {
        streamId: first.streamId,
      });
      if (next.chunk.lines.length === 0) break;
      lines.push(...next.chunk.lines);
    }
  }
  return {
    functions: first.functionBodyOffsets.length / 2,
    total_lines: first.totalNumberOfLines,
    first_lines: lines.slice(0, 24),
    full_text: full ? lines.join('\n') + '\n' : undefined,
  };
}

function replay(exports, seed, actions) {
  const bytes = new TextEncoder().encode(JSON.stringify({ seed, actions }));
  exports.shuyi_input_clear();
  for (const byte of bytes) {
    if (exports.shuyi_input_push(byte) !== 0) {
      throw new Error('Wasm request buffer overflow');
    }
  }
  if (exports.shuyi_run() !== 0) {
    throw new Error('Wasm replay failed');
  }
  const len = exports.shuyi_output_len();
  const output = new Uint8Array(len);
  for (let i = 0; i < len; i += 1) {
    const byte = exports.shuyi_output_byte(i);
    if (byte > 255) throw new Error(`Wasm output ended at byte ${i}`);
    output[i] = byte;
  }
  return JSON.parse(new TextDecoder().decode(output));
}

async function main() {
  const args = process.argv.slice(2);
  if (args.length !== 3 && !(args.length === 5 && args[3] === '--disasm-out')) {
    throw new Error('usage: node inspector-trace.mjs <fixture.wasm> <u64-seed> <comma-actions|-> [--disasm-out <path>]');
  }
  const [path, seed, actionText] = args;
  if (!/^\d+$/.test(seed)) throw new Error('seed must be an unsigned decimal integer');
  const actions = actionText === '-' ? [] : actionText.split(',').map((part) => part.trim());
  const bytes = await readFile(path);
  const session = new inspector.Session();
  session.connect();
  const wasmScripts = [];
  const breakpointHits = [];
  session.on('Debugger.scriptParsed', (message) => {
    if (message.params.url.startsWith('wasm://')) {
      wasmScripts.push({ id: message.params.scriptId, url: message.params.url });
    }
  });
  session.on('Debugger.paused', (message) => {
    const frame = message.params.callFrames[0];
    breakpointHits.push({
      reason: message.params.reason,
      function: frame?.functionName,
      location: frame?.location,
    });
    session.post('Debugger.resume');
  });
  try {
    progress('enabling debugger');
    await post(session, 'Runtime.enable');
    await post(session, 'Debugger.enable');
    await post(session, 'Profiler.enable');
    await post(session, 'Profiler.startPreciseCoverage', { callCount: true, detailed: true });
    progress('instantiating wasm');
    const instantiated = await WebAssembly.instantiate(bytes);
    globalThis.__shuyiInspectorRun = instantiated.instance.exports.shuyi_run;
    const evaluated = await post(session, 'Runtime.evaluate', {
      expression: 'globalThis.__shuyiInspectorRun',
    });
    await post(session, 'Debugger.setBreakpointOnFunctionCall', {
      objectId: evaluated.result.objectId,
    });
    progress('replaying');
    const result = replay(instantiated.instance.exports, seed, actions);
    progress('collecting coverage');
    delete globalThis.__shuyiInspectorRun;
    const coverage = await post(session, 'Profiler.takePreciseCoverage');
    const disassembly = [];
    for (const script of wasmScripts) {
      progress(`disassembling ${script.url}`);
      disassembly.push({
        url: script.url,
        ...(await disassemble(session, script.id, args.length === 5)),
      });
    }
    if (args.length === 5) {
      await writeFile(args[4], disassembly.map((part) => part.full_text).join('\n'));
    }
    const wasmCoverage = coverage.result
      .filter((item) => item.url.startsWith('wasm://'))
      .map((item) => ({
        url: item.url,
        functions: item.functions.length,
        executed_functions: item.functions.filter((fn) =>
          fn.ranges.some((range) => range.count > 0)).length,
      }));
    console.log(JSON.stringify({
      tool: 'Node V8 Inspector',
      node: process.version,
      wasm_sha256: createHash('sha256').update(bytes).digest('hex'),
      disassembly: disassembly.map(({ full_text: _fullText, ...part }) => part),
      breakpoint_hits: breakpointHits,
      wasm_coverage: wasmCoverage,
      wasm_coverage_available: wasmCoverage.length > 0,
      replay: result,
    }, null, 2));
    progress('done');
  } finally {
    session.disconnect();
  }
}

main().catch((error) => {
  console.error(error);
  process.exitCode = 1;
});
