# brine-kiwi

This is a Rust-native implementation of the Kiwi schema, decoder, encoder, compiler and rust code generator. 

Kiwi is a schema-based binary format for efficiently encoding trees of data.
It's inspired by Google's [Protocol Buffer](https://developers.google.com/protocol-buffers/) format but is simpler, has a more compact encoding, and has better support for optional fields.

> [!NOTE]
> Kiwi was originally designed by [Evan Wallace](https://madebyevan.com/figma/) for [Figma](https://www.figma.com/). 
>
> This Rust re-implementation (brine‐kiwi) is a fully‐native Rust compiler and SDK. This crate provides a runtime SDK, a schema compiler (including Rust code generation), and a CLI.

## Key Features

- **Efficient encoding of common values:** Numeric types use variable-length encoding so small values take fewer bytes.  
- **Efficient encoding of compound objects:** Structs allow nested objects with zero overhead for absent data.  
- **Detectable optional fields:** Messages can tell when a field is missing—unlike Protocol Buffers for repeated fields.  
- **Linearly serializable:** Read/write in a single pass for cache efficiency and guaranteed time complexity.  
- **Backwards compatibility:** New schema versions can read old data.  
- **Forwards compatibility:** Old schema versions can skip unknown fields when a copy of the new schema is present.  
- **Simple implementation:** Minimal API; the generated C++ code depends on a single file.


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

See the generated rust code [here](https://github.com/zfedoran/brine-kiwi/blob/main/example/src/generated.rs).



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

