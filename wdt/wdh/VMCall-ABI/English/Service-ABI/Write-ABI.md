# `Write` — Service 1
`Write` writes a string from VM memory to a standard stream of the host process.
The stream is selected via arguments.

## Format
```text
VMCALL 1, <address>, <size>
```

## Arguments
The block represents the string to be output, with a stream byte and an encoding byte at the beginning:

| Offset | Size    | Field    | Value                              |
|--------|---------|----------|------------------------------------|
| 0      | 1       | stream   | 0 = stdin, 1 = stdout, 2 = stderr  |
| 1      | 1       | encoding | string encoding                    |
| 2      | size -2 | data     | string bytes in the specified encoding |

- The block has no fixed size: the length is specified by the `size` operand of the `VMCALL` instruction.
- `size < 2` — the header is incomplete, and the host cannot decode the data: argument format error.
- Writing to stdin is not permitted; therefore, `stream == 0` results in an error.
- The host converts the string to UTF-8 and writes it to the stream.
- The host flushes the buffer after every call: output is visible immediately, and interleaving with other streams is predictable.

## Encodings
The encoding byte is the second byte of the block:

| Value | Encoding  | Status      |
|-------|-----------|-------------|
| 0     | ASCII     | supported   |
| 1     | UTF-8     | supported   |
| 2     | UTF-16 LE | supported   |
