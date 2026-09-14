# W8 Bytecode Format (`.wb`)
This document describes the binary format used to store W8 bytecode for execution by the W8 virtual machine.

## Contents
- [Byte Order](#byte-order)
- [W8 Bytecode File Structure](#w8-bytecode-file-structure)
- [Magic Signature](#magic-signature)
- [W8 Version](#w8-version)
- [Bytecode](#bytecode)

## Byte Order
All multi-byte integer values in the W8 Bytecode format are stored in **Little-Endian** byte order.

---

## W8 Bytecode File Structure
| Offset | Size    | Section     |
|:------:|:-------:|-------------|
| 0      | 5 bytes | Magic       |
| 5      | 6 bytes | W8 Version  |
| 11     | —       | Bytecode    |

---

## Magic Signature
The first 5 bytes of the file must be:
```text
4E 56 4D 42 43
```
(`NVMBC`)

This signature identifies the file as an W8 Bytecode file.

---

## W8 Version
Immediately following the magic signature are **6 bytes** specifying the version of the W8 with which the file was compiled.

The version is stored as three consecutive `u16` values.
```text
<u16><u16><u16>
```

W8 versions are incompatible with each other: the version stored in the file must exactly match the version of the virtual machine. Otherwise loading the file must fail with a version compatibility error.

---

## Bytecode
The file header is immediately followed by a stream of instructions.

### Instruction Encoding
```
[opcode: u8]                 — OperationCode (see opcode.rs)
[operand_count: u8]          — number of operands, 0–3
[operand₁]                   — if count ≥ 1
[operand₂]                   — if count ≥ 2
[operand₃]                   — if count ≥ 3
```

### Operand Encoding
Each operand starts with a 1-byte tag followed by its data:

| Tag  | Type      | Data                                  |
|:----:|-----------|---------------------------------------|
| 0x00 | Register  | 1 byte — register number (`u8`)       |
| 0x01 | Immediate | 8 bytes — value (`u64` Little-Endian) |
