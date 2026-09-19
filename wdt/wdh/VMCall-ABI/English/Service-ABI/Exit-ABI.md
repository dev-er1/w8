# `Exit` — Service 0
`Exit` terminates the VM with an exit code. This code is propagated to the caller; for instance, `w8c run` terminates with the same code.

## Format
```text
VMCALL 0, <address>, 1
```

## Arguments
The block is exactly 1 byte long: the number of the register holding the exit code.

| Offset | Size | Field           | Value                  |
|--------|------|-----------------|------------------------|
| 0      | 1    | Register number | Register with exit code |

## Exit Code
The code is read from the register at the time of the `VMCALL` invocation. `0` indicates success, while any other value indicates an error.
