# `IsInTerminal` — Service 2
`IsInTerminal` checks whether the host process's standard stream is connected to a terminal.
The result is written to a VM register.

## Format
```text
VMCALL 2, <address>, 2
```

## Arguments
The block is exactly 2 bytes long: the register for the result and the stream selector.

| Offset | Size | Field    | Value                             |
|--------|------|----------|-----------------------------------|
| 0      | 1    | register | Register for the result           |
| 1      | 1    | stream   | 0 = stdin, 1 = stdout, 2 = stderr |

## Result
The host writes `1` to the register if the stream is connected to a terminal, and `0` otherwise.
The register should be read after the `VMCALL` returns. The service always returns a `Continue` decision.
