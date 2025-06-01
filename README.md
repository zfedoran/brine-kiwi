# brine-kiwi

**Kiwi** (“brine-kiwi”) is a schema-based binary format for efficiently encoding trees of data. It’s inspired by Protocol Buffers but offers simpler, more compact encoding with first-class support for optional fields. 

This crate provides a complete native-Rust implementation: a runtime SDK, a schema compiler (including Rust code generation), and a CLI.

## Key Features

- **Efficient encoding of common values:** Numeric types use variable-length encoding so small values take fewer bytes.  
- **Efficient encoding of compound objects:** Structs allow nested objects with zero overhead for absent data.  
- **Detectable optional fields:** Messages can tell when a field is missing—unlike Protocol Buffers for repeated fields.  
- **Linearly serializable:** Read/write in a single pass for cache efficiency and guaranteed time complexity.  
- **Backwards compatibility:** New schema versions can read old data.  
- **Forwards compatibility:** Old schema versions can skip unknown fields when a copy of the new schema is present.  
- **Simple implementation:** Minimal API; the generated C++ code depends on a single file.

## Non‐goals

- **Optimal bit‐packing:** Post‐encoding compression can be applied if more space savings are needed.

## Quickstart

1. **Build everything**  
   Change into the workspace root and run:
   ```
   cargo build
   cargo install --path cli
   ```

2. **Compile a schema to binary**  
   ```
   bkiwi compile -i path/to/schema.kiwi -o path/to/schema.kiwi.bin
   ```

3. **Decode a binary to JSON**  
   ```
   bkiwi decode -i path/to/schema.kiwi.bin
   ```

4. **Generate Rust code**  
   ```
   bkiwi gen-rust -i path/to/schema.kiwi -o path/to/generated.rs
   ```

## Native Types

- **bool** (1 byte)  
- **byte** (u8, 1 byte)  
- **int** (i32 varint, ≤5 bytes)  
- **uint** (u32 varint, ≤5 bytes)  
- **float** (f32, 4 bytes; zero encodes as 1 byte)  
- **string** (UTF-8, null-terminated)  
- **int64** (i64 varint, ≤9 bytes)  
- **uint64** (u64 varint, ≤9 bytes)  
- **T[]** (array of any type)

## User Types

- **enum**: Named variants backed by a uint.  
- **struct**: Fixed, required fields in order (no additions once in use).  
- **message**: Optional fields; new fields can be added without breaking older readers.

## Example Schema

```proto
enum Type {
  FLAT = 0;
  ROUND = 1;
  POINTED = 2;
}

struct Color {
  byte red;
  byte green;
  byte blue;
  byte alpha;
}

message Example {
  uint clientID = 1;
  Type type = 2;
  Color[] colors = 3;
}
```

You can compile this schema to binary or generate Rust code using the `bkiwi` CLI tool.

## Live Demo

See [http://evanw.github.io/kiwi/](http://evanw.github.io/kiwi/) for a live demo of the schema compiler.


## Examples

- **Compile schema.kiwi**  
  ```
  bkiwi compile -i example/simple.kiwi -o example/simple.kiwi.bin
  ```

- **Decode to JSON**  
  ```
  bkiwi decode -i example/simple.kiwi.bin
  ```

- **Generate Rust bindings**  
  ```
  bkiwi gen-rust -i example/simple.kiwi -o example/src/generated.rs
  ```

- **Run example-app**  
  The `example/` directory contains a small Rust binary that:
    1. Builds a `kiwi_schema::Value` by hand matching the schema.  
    2. Calls `Example::from_kiwi(&value)` on the generated types.  
    3. Prints out the resulting fields.  
  To run it (after generating `generated.rs`):
  ```
  cd example
  ./run-example.sh
  ```

## Acknowledgments
Kiwi was originally designed by `Evan Wallace` for `Figma`. This Rust reimplementation (brine‐kiwi) is a fully‐native Rust compiler and SDK.
