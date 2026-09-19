# `TerminalSize` — Service 3
`TerminalSize` returns the terminal size in characters.
The size is written to two VM registers.

## Format
```text
VMCALL 3, <address>, 3
```

## Arguments
The block is exactly 3 bytes: the stream selector and the numbers of the two registers.

| Offset | Size | Field  | Value                                   |
|--------|------|--------|-----------------------------------------|
| 0      | 1    | stream | 0 = stdin, 1 = stdout, 2 = stderr       |
| 1      | 1    | width  | register for terminal width             |
| 2      | 1    | height | register for terminal height            |

## Result
The host writes the terminal size in characters to the registers. The size is read after returning from `VMCALL`. If the stream has no terminal,
`0` is written to both registers. The service always returns a `Continue` decision.
