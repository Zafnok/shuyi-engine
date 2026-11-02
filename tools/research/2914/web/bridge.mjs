// Browser and Node transport for the research fixture's generator-free Wasm ABI.

const encoder = new TextEncoder();
const decoder = new TextDecoder();

function actionsFrom(value) {
  if (Array.isArray(value)) return value.map(String);
  if (value === "" || value === "-") return [];
  return String(value).split(",").map((action) => action.trim());
}

function seedFrom(value) {
  if (typeof value === "number" && !Number.isSafeInteger(value)) {
    throw new RangeError("Seed numbers must be safe integers; pass larger seeds as decimal strings");
  }
  return String(value);
}

export async function createFixture(source) {
  const bytes = source instanceof Uint8Array
    ? source
    : source instanceof ArrayBuffer
      ? new Uint8Array(source)
      : new Uint8Array(await (await fetch(source)).arrayBuffer());
  const { instance } = await WebAssembly.instantiate(bytes, {});
  const wasm = instance.exports;
  for (const name of [
    "shuyi_input_clear", "shuyi_input_push", "shuyi_run",
    "shuyi_output_len", "shuyi_output_byte",
  ]) {
    if (typeof wasm[name] !== "function") throw new Error(`Missing Wasm export: ${name}`);
  }

  return {
    replay(seed, actions) {
      const request = encoder.encode(JSON.stringify({
        seed: seedFrom(seed),
        actions: actionsFrom(actions),
      }));
      wasm.shuyi_input_clear();
      for (const byte of request) {
        if (wasm.shuyi_input_push(byte) !== 0) throw new Error("Wasm request exceeds its byte limit");
      }
      const status = wasm.shuyi_run();
      const length = wasm.shuyi_output_len();
      const output = new Uint8Array(length);
      for (let index = 0; index < length; index += 1) {
        const byte = wasm.shuyi_output_byte(index);
        if (byte > 255) throw new Error("Wasm response ended early");
        output[index] = byte;
      }
      const json = decoder.decode(output);
      if (status !== 0) {
        const error = JSON.parse(json);
        throw new Error(error.error ?? json);
      }
      return json;
    },
  };
}
