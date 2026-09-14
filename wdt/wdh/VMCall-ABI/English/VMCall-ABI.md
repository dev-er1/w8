# `VMCALL` ABI
This document describes the purpose of the `VMCALL` opcode and the definition of the term “host”.

## What `VMCALL` Does
`VMCALL` is a three-operand opcode, analogous to the x64 `syscall` instruction. Execution proceeds as follows:

1. The VM verifies that the argument block lies entirely within the VM's memory.
2. The VM transfers control to the host, passing the service number and the argument block.
3. The host executes the service and returns a decision: to continue execution or to terminate the VM with an exit code.
4. The VM applies the decision.

## Instruction Format
```text
VMCALL <service>, <address>, <size>
```

- `service` — service number;
- `address` — address of the argument block in memory;
- `size` — size of the block in bytes.

The block must lie entirely within the VM's memory: `address + size` must not exceed its boundaries

## What Is a “Host”?
A host is a wrapper around the VM that owns the service table, as well as the associated ABIs and error definitions. The default host is WDH;
its services are described in [`Service-ABI/`](Service-ABI/). A different host may define its own service table and ABI.

## Services
The numbering and argument formats for `wdh` host services are described in [`Service-ABI/`](Service-ABI/).
