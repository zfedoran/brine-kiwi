# Example: Using the brine-kiwi Workspace

This folder contains a minimal Kiwi schema (`simple.kiwi`) and a script (`run-example.sh`) that demonstrates how to:

1. **Compile** the `.kiwi` text file into a binary `.kiwi.bin`  
2. **Generate** a Rust source file from that same `.kiwi` schema

## Contents

- `simple.kiwi`  
  A small schema defining an `enum Type`, a `struct Color`, and a `message Example`.

- `run-example.sh`  
  A helper script that:
  1. Runs `bkiwi compile -i simple.kiwi -o simple.kiwi.bin`  
  2. Runs `bkiwi gen-rust  -i simple.kiwi -o src/generated.rs`

- `src/generated.rs` (after you run the script)  
  The Rust code produced by `compile_schema_to_rust(simple.kiwi)`.

- `simple.kiwi.bin` (after you run the script)  
  The binary `.kiwi` representation produced by `compile_schema`.

## How to run

From the workspace root, do:

```bash
cd example

# Ensure the script is executable (only needed once)
chmod +x run-example.sh

# Run the demo (CLI binary named "bkiwi")
./run-example.sh
```
