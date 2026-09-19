// w8-core/src/vm/interpreter/macros.rs
//

/// Reads the value of an operand:
/// - `reg` — the register contents;
/// - `imm` — the immediate from the slot.
#[macro_export]
macro_rules! read_operand {
    ($vm:expr, $p:expr, $n:expr, reg) => {
        read_reg($vm, $p, $n)
    };
    ($vm:expr, $p:expr, $n:expr, imm) => {
        read_imm($p, $n)
    };
}

/// Generator of a `VMCALL` variant (3 operands: service, address, size — any).
#[macro_export]
macro_rules! vmcall_variant {
    ($name:ident, $k1:tt, $k2:tt, $k3:tt) => {
        pub(crate) fn $name(vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            let service = read_operand!(vm, p, 0, $k1);
            let address = read_operand!(vm, p, 1, $k2) as usize;
            let size = read_operand!(vm, p, 2, $k3) as usize;

            // The arguments block `address..address + size` must lie
            // entirely within the memory (as with the `LOAD*`/`STORE*`
            // addressing) — a trap on violation.
            let end = address.checked_add(size).ok_or_else(|| {
                VMError::new(VMErrorKind::InvalidAddress {
                    got: address,
                    memory_length: vm.memory.len(),
                })
            })?;
            if end > vm.memory.len() {
                return Err(VMError::new(VMErrorKind::InvalidAddress {
                    got: address,
                    memory_length: vm.memory.len(),
                }));
            }

            // The host is taken out of the VM for the duration of the call:
            // it receives `&mut WVM` and may mutate the VM (including the
            // dispatcher itself).
            let mut host = vm.dispatch.take();
            let decision = match host.as_mut() {
                Some(handler) => handler(vm, service, address as u64, size as u64),
                None => return Err(VMError::new(VMErrorKind::UnknownVMCallService { service })),
            };
            vm.dispatch = host;

            match decision {
                VMCallDecision::Continue => Ok(ip + 1),
                VMCallDecision::Exit { code } => {
                    vm.exit_code = Some(code);
                    Ok(EXIT_MARKER)
                }
            }
        }
    };
}

/// Generator of a `MOVE` variant (2 operands: dst — register, src — any).
#[macro_export]
macro_rules! move_variant {
    ($name:ident, $k:tt) => {
        pub(crate) fn $name(vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            vm.registers[Register(slot(p, 0) as u8)] = read_operand!(vm, p, 1, $k);
            Ok(ip + 1)
        }
    };
}

/// Generator of a `LOAD*` variant (2 operands: dst — register, address — any).
#[macro_export]
macro_rules! load_variant {
    ($name:ident, $method:ident, $k:tt) => {
        pub(crate) fn $name(vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            let address = read_operand!(vm, p, 1, $k) as usize;
            let value = vm.memory.$method(address).ok_or_else(|| {
                VMError::new(VMErrorKind::InvalidAddress {
                    got: address,
                    memory_length: vm.memory.len(),
                })
            })?;
            vm.registers[Register(slot(p, 0) as u8)] = u64::from(value);
            Ok(ip + 1)
        }
    };
}

/// Generator of a `STORE*` variant (2 operands: address and value — any).
#[macro_export]
macro_rules! store_variant {
    ($name:ident, $method:ident, $cast:ty, $ka:tt, $kv:tt) => {
        pub(crate) fn $name(vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            let address = read_operand!(vm, p, 0, $ka) as usize;
            let value = read_operand!(vm, p, 1, $kv) as $cast;
            vm.memory.$method(address, value).ok_or_else(|| {
                VMError::new(VMErrorKind::InvalidAddress {
                    got: address,
                    memory_length: vm.memory.len(),
                })
            })?;
            Ok(ip + 1)
        }
    };
}

/// Generator of a binary operation variant (3 operands: dst — register,
/// `src1` and `src2` — any). `$op` — a closure `|lhs, rhs| ...` over `u64`
/// (bit conversion to `f64` and back — inside the closure).
#[macro_export]
macro_rules! binary_variant {
    ($name:ident, $k1:tt, $k2:tt, $op:expr) => {
        pub(crate) fn $name(vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            let lhs = read_operand!(vm, p, 1, $k1);
            let rhs = read_operand!(vm, p, 2, $k2);
            vm.registers[Register(slot(p, 0) as u8)] = $op(lhs, rhs);
            Ok(ip + 1)
        }
    };
}

#[macro_export]
macro_rules! binops {
    ($op:expr, $($name:ident: $k1:tt $k2:tt),+ $(,)?) => {
        $(binary_variant!($name, $k1, $k2, $op);)+
    };
}

/// Generator of a division/remainder variant — like `binary_variant`, but
/// with a divisor zero check.
#[macro_export]
macro_rules! division_variant {
    ($name:ident, $k1:tt, $k2:tt, $op:expr) => {
        pub(crate) fn $name(vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            let rhs = read_operand!(vm, p, 2, $k2);
            ensure_nonzero_divisor(rhs)?;
            let lhs = read_operand!(vm, p, 1, $k1);
            vm.registers[Register(slot(p, 0) as u8)] = $op(lhs, rhs);
            Ok(ip + 1)
        }
    };
}

#[macro_export]
macro_rules! divisions {
    ($op:expr, $($name:ident: $k1:tt $k2:tt),+ $(,)?) => {
        $(division_variant!($name, $k1, $k2, $op);)+
    };
}

/// Generator of a unary operation variant (2 operands: dst — register,
/// src — any).
#[macro_export]
macro_rules! unary_variant {
    ($name:ident, $k:tt, $op:expr) => {
        pub(crate) fn $name(vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            let value = read_operand!(vm, p, 1, $k);
            vm.registers[Register(slot(p, 0) as u8)] = $op(value);
            Ok(ip + 1)
        }
    };
}

#[macro_export]
macro_rules! unaries {
    ($op:expr, $($name:ident: $k:tt),+ $(,)?) => {
        $(unary_variant!($name, $k, $op);)+
    };
}

/// Generator of an unconditional jump variant (1 operand — the target).
#[macro_export]
macro_rules! jmp_variant {
    ($name:ident, $k:tt) => {
        pub(crate) fn $name(_vm: &mut WVM, p: *const u64, _ip: usize) -> HandlerResult {
            let target = read_operand!(_vm, p, 0, $k) as usize;
            Ok(target)
        }
    };
}

/// Generator of a conditional jump variant (2 operands:
/// the condition and the target — any). `$taken` — a closure `|cond| bool`.
#[macro_export]
macro_rules! cond_variant {
    ($name:ident, $k1:tt, $k2:tt, $taken:expr) => {
        pub(crate) fn $name(_vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            let cond = read_operand!(_vm, p, 0, $k1);
            let target = read_operand!(_vm, p, 1, $k2) as usize;
            if $taken(cond) { Ok(target) } else { Ok(ip + 1) }
        }
    };
}

/// Generator of a `CALL` variant (1 operand — the target).
#[macro_export]
macro_rules! call_variant {
    ($name:ident, $k:tt) => {
        pub(crate) fn $name(vm: &mut WVM, p: *const u64, ip: usize) -> HandlerResult {
            let target = read_operand!(vm, p, 0, $k) as usize;
            // The return address is the instruction following `CALL`.
            vm.call_stack.push(ip + 1);
            Ok(target)
        }
    };
}
