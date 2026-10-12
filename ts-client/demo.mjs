// The Trust Layer — minimal TS client.
// Uses web3.js + the Anchor IDL (the same path a real user would take).
// Default target: devnet program. For a local run against a validator,
// set ANCHOR_PROVIDER_URL=http://localhost:8899.
import anchor from "@coral-xyz/anchor";
import { Connection, Keypair, LAMPORTS_PER_SOL, PublicKey, SystemProgram } from "@solana/web3.js";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";

const IDL = JSON.parse(readFileSync(new URL("./idl/trust_layer.json", import.meta.url), "utf8"));
const PROGRAM_ID = new PublicKey("AYqDidei67kRrTtJ4jqpG9iFJ9UfUg16ZN5mBEZj3sne");
const CLUSTER = process.env.ANCHOR_PROVIDER_URL ?? "https://api.devnet.solana.com";

const connection = new Connection(CLUSTER, "confirmed");
const wallet = Keypair.generate(); // demo issuer
const provider = new anchor.AnchorProvider(connection, new anchor.Wallet(wallet), {
  commitment: "confirmed",
});
const program = new anchor.Program(IDL, provider);

const sha256 = (buf) => new Uint8Array(createHash("sha256").update(buf).digest());

function receiptPda(payloadHash) {
  return PublicKey.findProgramAddressSync(
    [Buffer.from("receipt"), Buffer.from(payloadHash)],
    PROGRAM_ID,
  );
}

async function main() {
  const payload = Buffer.from(
    JSON.stringify({ deliverable: "q3-report.pdf", from: "acme", to: "globex", amount: 1200 }),
  );
  const hash = sha256(payload);
  const [pda] = receiptPda(hash);
  console.log(`cluster:        ${CLUSTER}`);
  console.log(`program:        ${PROGRAM_ID.toString()}`);
  console.log(`issuer:         ${wallet.publicKey.toString()}`);
  console.log(`payload hash:   ${Buffer.from(hash).toString("hex")}`);
  console.log(`receipt PDA:    ${pda.toString()}`);

  // 1) ISSUE -------------------------------------------------------------
  console.log("\n[1] issue_receipt ...");
  try {
    const sig = await program.methods
      .issueReceipt([...hash], "ts-client demo")
      .accounts({ receipt: pda, issuer: wallet.publicKey, systemProgram: SystemProgram.programId })
      .signers([wallet])
      .rpc();
    console.log(`    issued! tx: https://explorer.solana.com/tx/${sig}?cluster=custom&customUrl=${CLUSTER}`);
  } catch (e) {
    console.log(`    issue failed (already issued or unfunded wallet): ${e.message?.slice(0, 160)}`);
  }

  // 2) VERIFY (untouched payload) ----------------------------------------
  console.log("\n[2] verify_receipt (untouched payload) ...");
  try {
    const res = await program.methods
      .verifyReceipt([...hash])
      .accounts({ receipt: pda })
      .view();
    console.log("    result:", JSON.stringify(res, (k, v) => (typeof v === "object" && v?.type === "Buffer" ? Buffer.from(v.data).toString("hex") : v)));
    console.log(`    matches=${res.matches} issuer=${res.issuer?.toString?.() ?? res.issuer} slot=${res.slot}`);
  } catch (e) {
    console.log(`    verify failed: ${e.message?.slice(0, 160)}`);
  }

  // 3) TAMPER: flip one byte, verify again -> must FAIL -------------------
  console.log("\n[3] tamper: flip one byte of the payload, verify again ...");
  const tampered = Buffer.from(payload);
  tampered[10] ^= 0x01;
  const tamperedHash = sha256(tampered);
  const [tamperedPda] = receiptPda(tamperedHash);
  try {
    const res = await program.methods
      .verifyReceipt([...tamperedHash])
      .accounts({ receipt: tamperedPda })
      .view();
    console.log(`    UNEXPECTED SUCCESS — this must not happen: ${JSON.stringify(res)}`);
    process.exitCode = 1;
  } catch (e) {
    console.log("    TAMPER DETECTED — verify failed as expected:");
    console.log(`    ${String(e.message ?? e).split("\n").slice(0, 3).join("\n    ")}`);
  }
}

main().then(
  () => process.exit(),
  (e) => {
    console.error(e);
    process.exit(1);
  },
);
