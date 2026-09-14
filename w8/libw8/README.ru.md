# `libw8`
`libw8` — крейт для использования виртуальной машины W8. `libw8` представляет высокоуровневый API для запуска кода.

## API
- **`W8Assembler`** — компиляция W8 Assembly в инструкции или байткод (`.wb`):
  - `assemble(source)` и `assemble_from_path(path)` -> `Vec<Instruction>`
  - `assemble_to_bytecode(source)` и `assemble_to_bytecode_from_path(path)` -> `Vec<u8>`
  - функции с `_from_path` поддерживают директиву `#include`
- **`W8`** — выполнение байткода:
  - `run(BytecodeSource)` — без хоста
  - `interpretate_with(BytecodeSource, dispatch)` — с хостом-диспетчером для `VMCALL`
  - `W8::new()` и `with_memory_size(bytes)` — размер памяти ВМ
  - возвращают `Result<Option<u64>, WVMError>`: `Some(code)` — программа завершилась через сервис `Exit`; `None` — дошла до конца без явного выхода
- **`BytecodeSource`** — источник байткода: `File(PathBuf)` | `Bytes(Vec<u8>)` | `Instructions(Vec<Instruction>)`
- **`DEFAULT_MEMORY_SIZE`** — размер памяти ВМ по умолчанию (64 КБ)
- Реэкспорты: `WVM`, `VMCallDecision`, `Instruction`, `W8_VERSION`, `WVMError`, `WVMErrorKind`, `W8AsmError`, `W8AsmErrorKind`

## Подключение
```toml
[dependencies]
libw8 = { path = "../w8/libw8" }
```

## Пример
Программа пишет «Hi» в stdout (сервис 1 — `Write`) и завершается с кодом 0 (сервис 0 — `Exit`):

```rust
use libw8::{BytecodeSource, W8Assembler, W8};
use libw8::{WVM, VMCallDecision};
use w8_core::isa::register::Register;

use std::io::Write;

fn main() {
    // 1. Компилируем W8 Assembly.
    let instructions = W8Assembler::assemble("
        store8 0, 1      ; блок Write: [поток][кодировка][H][i][\\n]
        store8 1, 0      ; кодировка: ASCII
        store8 2, 72     ; 'H'
        store8 3, 105    ; 'i'
        store8 4, 10     ; '\\n'
        vmcall 1, 0, 5   ; Write: вывести memory[0..5]

        move r7, 0       ; код выхода = 0
        move r6, 7       ; номер регистра с кодом выхода
        store8 1024, r6  ; блок Exit: memory[1024] = 7
        vmcall 0, 1024, 1; Exit
    ")
    .expect("valid program");

    // 2. Выполняем программу со своим хостом-диспетчером: он получает
    //    `&mut WVM`, номер сервиса и блок аргументов в памяти ВМ.
    let exit_code = W8::new()
        .interpretate_with(
            BytecodeSource::Instructions(instructions),
            |vm: &mut WVM, service, address, size| match service {
                // Write: блок = [поток][кодировка][data] — выводим data.
                1 => {
                    let block =
                        &vm.memory.as_slice()[address as usize..(address + size) as usize];
                    std::io::stdout().write_all(&block[2..]).expect("write");
                    VMCallDecision::Continue
                }
                // Exit: блок = номер регистра с кодом выхода.
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

## Состояние `libw8` на crates.io
Пока что `libw8` нету на *[crates.io](https://crates.io)*. Если вы хотите публикации `libw8` в crates.io — сделайте issue об
этом.
