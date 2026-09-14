# `libw8`
`libw8` is the crate for using the W8 virtual machine. `libw8` provides a high-level API for running code.

## API
- **`W8Assembler`** — compiles W8 Assembly into instructions or bytecode (`.wb`):
  - `assemble(source)` / `assemble_from_path(path)` -> `Vec<Instruction>`
  - `assemble_to_bytecode(source)` / `assemble_to_bytecode_from_path(path)` -> `Vec<u8>`
  - the `_from_path` functions support the `#include` directive
- **`W8`** — bytecode execution:
  - `run(BytecodeSource)` — without a host
  - `interpretate_with(BytecodeSource, dispatch)` — with a host dispatcher for `VMCALL`
  - `W8::new()` / `with_memory_size(bytes)` — the VM memory size
  - returns `Result<Option<u64>, WVMError>`: `Some(code)` — the program terminated via the `Exit` service; `None` — it ran off the end without an explicit exit
- **`BytecodeSource`** — the bytecode source: `File(PathBuf)` | `Bytes(Vec<u8>)` | `Instructions(Vec<Instruction>)`
- **`DEFAULT_MEMORY_SIZE`** — the default VM memory size (64 KB)
- Re-exports: `WVM`, `VMCallDecision`, `Instruction`, `W8_VERSION`, `WVMError`, `WVMErrorKind`, `W8AsmError`, `W8AsmErrorKind`

## Adding as a dependency
```toml
[dependencies]
libw8 = { path = "../w8/libw8" }
```

## Example
The program writes “Hi” to stdout (service 1 — `Write`) and terminates with the exit code 0 (service 0 — `Exit`):

```rust
use libw8::{BytecodeSource, W8Assembler, W8};
use libw8::{WVM, VMCallDecision};
// The host register type — from the `w8-core` crate.
use w8_core::isa::register::Register;

fn main() {
    // 1. Compile W8 Assembly.
    let instructions = W8Assembler::assemble("
        store8 0, 1      ; the Write block: [stream][encoding][H][i][\\n]
        store8 1, 0      ; the encoding: ASCII
        store8 2, 72     ; 'H'
        store8 3, 105    ; 'i'
        store8 4, 10     ; '\\n'
        vmcall 1, 0, 5   ; Write: write memory[0..5]

        move r7, 0       ; the exit code
        move r6, 7       ; the register number holding the exit code
        store8 1024, r6  ; the Exit block: memory[1024] = 7
        vmcall 0, 1024, 1; Exit
    ")
    .expect("valid program");

    // 2. Run the program with your own host dispatcher: it receives
    //    `&mut WVM`, the service number and the arguments block in the VM memory.
    let exit_code = W8::new()
        .interpretate_with(
            BytecodeSource::Instructions(instructions),
            |vm: &mut WVM, service, address, size| match service {
                // Write: the block = [stream][encoding][data] — write the data.
                1 => {
                    let block =
                        &vm.memory.as_slice()[address as usize..(address + size) as usize];
                    use std::io::Write;
                    std::io::stdout().write_all(&block[2..]).expect("write");
                    VMCallDecision::Continue
                }
                // Exit: the block = the register number holding the exit code.
                0 => {
                    let register = vm.memory.load_u8(address as usize).expect("valid block");
                    VMCallDecision::Exit {
                        code: vm.registers[Register(register)],
                    }
                }
                _ => VMCallDecision::Continue,
            },
        )
        .expect("valid bytecode");

    assert_eq!(exit_code, Some(0));
}
```

## `libw8` status on crates.io
Currently, `libw8` is not available on *[crates.io](https://crates.io)*. If you would like to see `libw8` published on crates.
io, please open an issue about it.
