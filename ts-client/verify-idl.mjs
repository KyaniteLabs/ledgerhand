// Proves the committed IDL is a usable contract for the Anchor TS client.
//
// Checks, in order:
//   1. `new anchor.Program(IDL, provider)` constructs without throwing.
//   2. Every instruction in the Rust `#[program]` module has an IDL entry whose
//      declared discriminator equals sha256("global:<name>")[:8] — the default
//      Anchor selector for that handler — and the instruction coder emits the
//      same 8 bytes when asked to encode the instruction.
//   3. Every `#[account]` struct has an IDL account entry whose discriminator
//      equals sha256("account:<name>")[:8].
//   4. The `VerificationResult` return type has fully typed fields and encodes.
//   5. The `Receipt` account round-trips through the account coder.
//
// The client camelCases every IDL name (`convertIdlToCamelCase`), so coder
// lookups use the camelCase form while the Rust/IDL parity checks use the
// committed snake_case names.
//
// Runs offline: no RPC is contacted. Exit code 0 = contract verified.
import anchor from "@coral-xyz/anchor";
import { Connection, Keypair, PublicKey } from "@solana/web3.js";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const here = new URL(".", import.meta.url);
const IDL = JSON.parse(readFileSync(new URL("./idl/trust_layer.json", here), "utf8"));
const RUST = readFileSync(new URL("../programs/trust_layer/src/lib.rs", here), "utf8");

const sha256 = (s) => createHash("sha256").update(s).digest();
const hex = (u8) => Buffer.from(u8).toString("hex");
const camel = (s) => s.replace(/_([a-z])/g, (_, c) => c.toUpperCase());
const failures = [];
const ok = (label, extra = "") => console.log(`PASS  ${label}${extra ? ` (${extra})` : ""}`);
const bad = (label, detail) => {
  failures.push(`${label}: ${detail}`);
  console.log(`FAIL  ${label}: ${detail}`);
};

// --- ground truth from the Rust program source ------------------------------
function programBody(src) {
  const start = src.indexOf("#[program]");
  if (start < 0) throw new Error("no #[program] module in lib.rs");
  const nextItem = src.indexOf("\n#[derive(Accounts)]", start);
  return src.slice(start, nextItem < 0 ? src.length : nextItem);
}
const rustInstructions = [...programBody(RUST).matchAll(/pub fn\s+(\w+)\s*\(\s*ctx:\s*Context</g)].map((m) => m[1]);
const rustAccounts = [...RUST.matchAll(/#\[account\][\s\S]{0,400}?pub struct\s+(\w+)/g)].map((m) => m[1]);

console.log(`rust instructions: ${rustInstructions.join(", ")}`);
console.log(`rust accounts:     ${rustAccounts.join(", ")}`);

// --- 1. Program construction ------------------------------------------------
const connection = new Connection("http://127.0.0.1:8899", "confirmed");
const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(Keypair.generate()), {
  commitment: "confirmed",
});
let program;
try {
  program = new anchor.Program(IDL, provider);
  ok("new anchor.Program(IDL, provider)");
} catch (e) {
  bad("new anchor.Program(IDL, provider)", `${e.constructor?.name}: ${e.message}`);
}

if (program) {
  // --- 2. instruction selectors --------------------------------------------
  const idlInstr = new Map(IDL.instructions.map((ix) => [ix.name, ix]));
  if (idlInstr.size !== rustInstructions.length) {
    bad("instruction set", `IDL has ${idlInstr.size}, Rust has ${rustInstructions.length}`);
  }
  const sampleArgs = {
    issue_receipt: [[...new Uint8Array(32)], "verify-idl"],
    verify_receipt: [[...new Uint8Array(32)]],
    void_receipt: [],
  };
  for (const name of rustInstructions) {
    const expected = hex(sha256(`global:${name}`).subarray(0, 8));
    const ix = idlInstr.get(name);
    if (!ix) {
      bad(`instruction ${name}`, "missing from committed IDL");
      continue;
    }
    const declared = ix.discriminator ? hex(ix.discriminator) : "(none)";
    declared === expected
      ? ok(`discriminator ${name}`, declared)
      : bad(`discriminator ${name}`, `IDL ${declared} != sha256("global:${name}")[:8] ${expected}`);
    if (!(name in sampleArgs)) continue;
    try {
      const data = program.coder.instruction.encode(camel(name), sampleArgs[name]);
      const emitted = hex(data.subarray(0, 8));
      emitted === expected
        ? ok(`coder emits ${name}`, emitted)
        : bad(`coder emits ${name}`, `${emitted} != ${expected}`);
    } catch (e) {
      bad(`coder emits ${name}`, `${e.constructor?.name}: ${e.message}`);
    }
  }
  for (const name of idlInstr.keys()) {
    if (!rustInstructions.includes(name)) bad(`instruction ${name}`, "IDL-only instruction has no Rust handler");
  }

  // The `program.methods.*` surface a client actually calls must exist too.
  for (const name of rustInstructions) {
    typeof program.methods[camel(name)] === "function"
      ? ok(`program.methods.${camel(name)} exists`)
      : bad(`program.methods.${camel(name)}`, "missing from Program");
  }

  // --- 3. account discriminators -------------------------------------------
  const idlAcct = new Map(IDL.accounts.map((a) => [a.name, a]));
  for (const name of rustAccounts) {
    const expected = hex(sha256(`account:${name}`).subarray(0, 8));
    const acct = idlAcct.get(name);
    if (!acct) {
      bad(`account ${name}`, "missing from committed IDL");
      continue;
    }
    const declared = acct.discriminator ? hex(acct.discriminator) : "(none)";
    declared === expected
      ? ok(`account discriminator ${name}`, declared)
      : bad(`account discriminator ${name}`, `IDL ${declared} != sha256("account:${name}")[:8] ${expected}`);
  }

  // --- 4. VerificationResult type is fully typed and encodable --------------
  const vrDef = IDL.types?.find((t) => t.name === "VerificationResult");
  if (!vrDef) {
    bad("type VerificationResult", "missing from committed IDL");
  } else {
    const untyped = vrDef.type.fields.filter((f) => !f.type).map((f) => f.name);
    untyped.length
      ? bad("type VerificationResult", `fields without a type: ${untyped.join(", ")}`)
      : ok("type VerificationResult fields", vrDef.type.fields.map((f) => `${f.name}<${JSON.stringify(f.type)}>`).join(", "));
    try {
      const encoded = program.coder.types.encode("verificationResult", {
        matches: false,
        receipt: null,
        issuer: null,
        slot: null,
        voided: false,
      });
      const decoded = program.coder.types.decode("verificationResult", encoded);
      const roundTrip = decoded.matches === false && decoded.voided === false && decoded.slot === null;
      roundTrip
        ? ok("type VerificationResult encode/decode", `${encoded.length} bytes`)
        : bad("type VerificationResult encode/decode", JSON.stringify(decoded));
    } catch (e) {
      bad("type VerificationResult encode/decode", `${e.constructor?.name}: ${e.message}`);
    }
  }

  // --- 5. Receipt account round-trip ---------------------------------------
  const receipt = {
    payloadHash: [...new Uint8Array(32).fill(7)],
    issuer: PublicKey.default,
    slot: new anchor.BN(42),
    memo: "verify-idl",
    authority: PublicKey.default,
    isVoided: false,
    bump: 255,
  };
  try {
    const data = await program.coder.accounts.encode("receipt", receipt);
    const expected = hex(sha256("account:Receipt").subarray(0, 8));
    if (hex(data.subarray(0, 8)) !== expected) {
      bad("Receipt account encode", `discriminator ${hex(data.subarray(0, 8))} != ${expected}`);
    }
    const decoded = program.coder.accounts.decode("receipt", data);
    decoded.memo === "verify-idl" && decoded.isVoided === false && decoded.slot.toNumber() === 42
      ? ok("Receipt account encode/decode", `${data.length} bytes`)
      : bad("Receipt account encode/decode", JSON.stringify(decoded));
  } catch (e) {
    bad("Receipt account encode/decode", `${e.constructor?.name}: ${e.message}`);
  }
}

if (failures.length) {
  console.error(`\n${failures.length} check(s) FAILED`);
  process.exit(1);
}
console.log("\nIDL contract verified: committed IDL constructs an Anchor client and matches the Rust handlers.");
