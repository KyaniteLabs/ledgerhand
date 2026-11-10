#!/usr/bin/env bash
# Regenerate ts-client/idl/trust_layer.json from the program source.
#
# The IDL is printed by the program itself: with `--features idl-build` Anchor
# generates private `__anchor_private_print_idl_*` tests that print the IDL
# between `--- IDL begin/end ... ---` markers. This script runs those tests and
# performs the same post-processing `anchor idl build` does (merge the address
# and errors blocks, strip module paths from type names).
#
# Two environment details are load-bearing:
#
#  1. ANCHOR_IDL_BUILD_RESOLUTION=TRUE must be set *before* the proc macro
#     expands. anchor-syn only emits IDL `pda` seeds, `address` and `relations`
#     when this is "TRUE" at macro time (anchor-syn-0.31.x
#     src/idl/accounts.rs, `option_env!("ANCHOR_IDL_BUILD_RESOLUTION")`).
#     `anchor idl build` exports it; a plain shell does not. Without it the
#     IDL is degraded — no PDA seeds and no system_program address — and Anchor
#     TS clients can no longer derive PDAs from it. This script exports it.
#
#  2. `anchor idl build` also passes
#     `RUSTFLAGS=--cfg procmacro2_semver_exempt`, which under anchor 0.31 makes
#     this crate's `#[derive(Accounts)]` expansion fail on a cold target
#     (`cannot find value payload_hash/receipt in this scope`). Running the
#     generator directly avoids that flag.
#
# Failure safety: the new IDL is written to a temp file and moved into place
# only after every step succeeds, so a failed run can never truncate the
# committed IDL. stderr is left visible so the failure is diagnosable.
#
# Usage: ./scripts/gen-idl.sh   (writes ts-client/idl/trust_layer.json)
set -euo pipefail
cd "$(dirname "$0")/.."

OUT=ts-client/idl/trust_layer.json
TMP="$(mktemp "${TMPDIR:-/tmp}/trust_layer-idl.XXXXXX")"
trap 'rm -f "$TMP"' EXIT

export ANCHOR_IDL_BUILD_RESOLUTION=TRUE

cargo test -p trust_layer --features idl-build __anchor_private_print_idl \
  -- --show-output --quiet | python3 -c '
import json
import re
import sys

raw = sys.stdin.read()

def block(name):
    m = re.search(r"--- IDL begin " + name + r" ---\n(.*?)\n--- IDL end " + name + r" ---", raw, re.S)
    if not m:
        sys.exit("could not find the %s IDL block in the test output" % name)
    return m.group(1)

prog = json.loads(block("program"))
# The address test prints the debug form of the program id, quotes included.
prog["address"] = json.loads(block("address")).strip("\"")
prog["errors"] = json.loads(block("errors"))

# Anchor writes the root fields in this order.
order = ["address", "metadata", "instructions", "accounts", "errors", "types"]
idl = {k: prog[k] for k in order if k in prog}

# `anchor idl build` strips module paths from names: trust_layer::Receipt -> Receipt.
def strip_module_paths(node):
    if isinstance(node, dict):
        return {
            k: (re.sub(r"^(\w+::)+", "", v) if k == "name" and isinstance(v, str) else strip_module_paths(v))
            for k, v in node.items()
        }
    if isinstance(node, list):
        return [strip_module_paths(v) for v in node]
    return node

sys.stdout.write(json.dumps(strip_module_paths(idl), indent=2, ensure_ascii=False))
' > "$TMP"

chmod 644 "$TMP"
mv "$TMP" "$OUT"
echo "wrote $OUT"
