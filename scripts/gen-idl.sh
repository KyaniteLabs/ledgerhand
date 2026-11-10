#!/bin/sh
# Regenerate ts-client/idl/trust_layer.json from the program source.
#
# The IDL is printed by the program itself: with `--features idl-build` Anchor
# generates private `__anchor_private_print_idl_*` tests that print the IDL
# between `--- IDL begin/end ... ---` markers. This script runs those tests and
# performs the same post-processing `anchor idl build` does (merge the address
# and errors blocks, strip module paths from type names).
#
# Why not `anchor idl build` directly: the CLI adds
# `RUSTFLAGS=--cfg procmacro2_semver_exempt`, which under anchor 0.31 makes this
# crate's `#[derive(Accounts)]` expansion fail on a cold target
# (`cannot find value payload_hash/receipt in this scope`). Running the
# generator directly avoids that flag, so the IDL cannot drift from source.
#
# Usage: ./scripts/gen-idl.sh   (writes ts-client/idl/trust_layer.json)
set -e
cd "$(dirname "$0")/.."

OUT=ts-client/idl/trust_layer.json

cargo test -p trust_layer --features idl-build __anchor_private_print_idl \
  -- --show-output --quiet 2>/dev/null | python3 -c '
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
' > "$OUT"

echo "wrote $OUT"
