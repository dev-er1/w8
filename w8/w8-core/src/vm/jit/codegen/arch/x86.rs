// w8-core/src/vm/jit/codegen/arch/x86.rs
//
//! *See [crate::vm::jit::codegen] module documentation to understand
//! meaning of the “codegen”.*
//!
//! # x86 codegen
//!
//! Machine code generator for the x86 CPU architecture.
//!
//! The overall design mirrors the x64 backend (see `arch::x64` for the
//! two-pass emission, the error stub, the epilogue, the address table,
//! and the `CALL`, `RET` and `VMCALL` trampoline scheme). Only what differs
//! on x86 is documented here.
//!
//! ## Register model
//!
//! Each W8 register is a 64-bit cell in host memory whose absolute
//! address is given by `self.register(index)`. On x86 a W8 value
//! is always handled as a **pair** of 32-bit halves:
//!
//! - `EAX` — the low 32 bits,
//! - `EDX` — the high 32 bits.
//!
//! The second operand of binary operations lives in `ECX` (low) and
//! `EBX` (high). `EBX` is callee-saved by the platform ABI, so the
//! generated function saves it (together with `ESI` and `EDI`) in the
//! prologue and restores it in the epilogue; every exit path
//! (fallthrough, error stub, out-of-range jump) goes through the
//! epilogue.
//!
//! ## Calling convention
//!
//! x86 uses `cdecl` on every OS, so — unlike x64 — there is a
//! single trampoline call sequence: arguments are moved into a stack
//! frame, `call eax` invokes the trampoline, and the frame is released
//! afterwards. A `u64` argument occupies two stack slots (low half
//! first); a `u64` outcome returns in `EDX:EAX`, and both halves are
//! folded together before an outcome is tested.
//!
//! The stack is kept 16-byte aligned at every `call`. The i386 System V
//! ABI hands the generated function `ESP % 16 == 12`; the prologue's three
//! pushes bring it to 0, and every block reserved afterwards — the
//! prologue's own, each call frame, each spill slot — is a multiple of 16,
//! so the alignment survives to every trampoline. This matters because the
//! trampolines are ordinary Rust functions and may use aligned SSE moves
//! on their own frames.
//!
//! ## Memory access
//!
//! `LOAD*` and `STORE*` re-read the memory base from the cell at
//! `ctx.membase_addr` (its *address* is baked in as an immediate, and the
//! cell is dereferenced exactly once) because a host callback may replace
//! the buffer mid-run.
//!
//! Like the x64 backend, the generated code does **not** bounds-check the
//! guest address against the memory length: the base and the offset are
//! simply added. Adding that check needs a memory-length cell in
//! [`GenContext`] alongside `membase`, which is a change to the shared
//! codegen interface rather than to this backend, so it is deliberately
//! left out of this file.
//!
//! ## Division
//!
//! 32-bit x86 has no 64-bit divide instruction, so `SDIV`, `UDIV`,
//! `SREM`, and `UREM` call small `extern "C"` helpers
//! ([`jit_sdiv64`], [`jit_udiv64`], [`jit_srem64`], [`jit_urem64`]) that
//! implement the exact interpreter semantics (`wrapping_div` or
//! `wrapping_rem` with a zero-divisor check). On division by zero they
//! report [`VmcallOut::DIVISION_BY_ZERO`] through the shared *out* block
//! and the generated code branches straight to the epilogue.
use crate::{
    isa::{
        instruction::Instruction, opcode::OperationCode, operand::OperandKind, register::Register,
    },
    vm::{
        VMCallDecision, WVM,
        err::{VMError, VMErrorKind},
        jit::codegen::{GenContext, GeneratedCode, JITCodegen, VmcallOut},
    },
};

/// A pending patch in the generated machine code, resolved in pass 2
/// (same scheme as the x64 backend; all address fields are 4 bytes here).
enum Fixup {
    /// A `rel32` field at `pos` jumping to the start of instruction `target`
    /// (out-of-range targets land on the epilogue).
    RelToInstr { pos: usize, target: usize },

    /// A `rel32` field at `pos` jumping to the epilogue.
    RelToEpilogue { pos: usize },

    /// A `rel32` field at `pos` jumping to the `RET`-empty error stub.
    RelToErrorStub { pos: usize },

    /// A 4-byte field at `pos` holding the address-table base.
    TableBase { pos: usize },

    /// A 4-byte field at `pos` holding the address-table length.
    TableLen { pos: usize },
}

/// Writes a `rel32` displacement jumping from the end of the 4-byte field at
/// `pos` to `target_off` (identical to the x64 backend).
fn patch_rel32(code: &mut [u8], pos: usize, target_off: usize) {
    let displacement = i32::try_from(target_off as i64 - (pos as i64 + 4))
        .expect("x86 jump displacement out of `i32` range");
    code[pos..pos + 4].copy_from_slice(&displacement.to_le_bytes());
}

/// Narrows a host address to the 32-bit form the generated code embeds.
///
/// Every address this backend bakes in (VM state, `ctx` cells, trampoline
/// entry points, address-table entries) is a host pointer, so on the only
/// target this module is built for the conversion is exact. The explicit
/// check turns a silent truncation into a loud failure should the file ever
/// be compiled for a wider address space.
fn addr32(address: u64) -> u32 {
    u32::try_from(address).expect("host address does not fit in 32 bits")
}

/// Whether the instruction jumps to a register-held target (same check as
/// the x64 backend).
fn has_register_target(instruction: &Instruction) -> bool {
    let target = match instruction.opcode {
        OperationCode::JMP | OperationCode::CALL => instruction.operand1,
        OperationCode::JZ | OperationCode::JNZ => instruction.operand2,
        _ => return false,
    };

    match target {
        Some(operand) => matches!(operand.kind, OperandKind::Register(_)),
        None => false,
    }
}

/// Pushes a return address onto the VM call stack (same contract as the
/// x64 backend).
extern "C" fn jit_call_push(stack: *mut Vec<usize>, ret_idx: usize) {
    // SAFETY: generated code passes the live VM call stack embedded at
    // codegen time; `run` does not return until the generated code finishes,
    // so the `Vec` outlives the call.
    unsafe {
        (*stack).push(ret_idx);
    }
}

/// Pops a return address from the VM call stack (same contract as the
/// x64 backend: writes the index to `out`, returns 1, or 0 when empty).
extern "C" fn jit_call_pop(stack: *mut Vec<usize>, out: *mut usize) -> u8 {
    // SAFETY: `stack` as above; `out` is the scratch cell owned by `JIT`,
    // valid for the whole execution.
    unsafe {
        match (*stack).pop() {
            Some(value) => {
                *out = value;
                1
            }
            None => 0,
        }
    }
}

/// Handles a `VMCALL` from generated code (same port of the interpreter's
/// `vmcall_variant!` as the x64 backend).
extern "C" fn jit_vmcall(
    wvm: *mut WVM,
    service: u64,
    address: u64,
    size: u64,
    out: *mut VmcallOut,
) -> u64 {
    // SAFETY: generated code passes the live VM and the `JIT`-owned block
    // embedded at codegen time; `run` does not return until the generated
    // code finishes, so both outlive the call.
    unsafe {
        let vm = &mut *wvm;
        let out = &mut *out;

        // The arguments block `address..address + size` must lie entirely
        // within the memory — a trap on violation, like the interpreter.
        let address = address as usize;
        let size = size as usize;
        let end = match address.checked_add(size) {
            Some(end) => end,
            None => {
                out.status = VmcallOut::INVALID_ADDRESS;
                out.info0 = address as u64;
                out.info1 = vm.memory.len() as u64;
                return VmcallOut::INVALID_ADDRESS;
            }
        };
        if end > vm.memory.len() {
            out.status = VmcallOut::INVALID_ADDRESS;
            out.info0 = address as u64;
            out.info1 = vm.memory.len() as u64;
            return VmcallOut::INVALID_ADDRESS;
        }

        // The host is taken out of the VM for the duration of the call.
        let mut host = vm.dispatch.take();
        let decision = match host.as_mut() {
            Some(handler) => handler(vm, service, address as u64, size as u64),
            None => {
                vm.dispatch = host;
                out.status = VmcallOut::UNKNOWN_SERVICE;
                out.info0 = service;
                return VmcallOut::UNKNOWN_SERVICE;
            }
        };
        vm.dispatch = host;

        out.membase = vm.memory.as_slice().as_ptr() as u64;

        match decision {
            VMCallDecision::Continue => {
                out.status = VmcallOut::OK;
                VmcallOut::OK
            }
            VMCallDecision::Exit { code } => {
                vm.exit_code = Some(code);
                out.status = VmcallOut::VMCALL_EXIT;
                VmcallOut::VMCALL_EXIT
            }
        }
    }
}

extern "C" fn jit_sdiv64(lhs: u64, rhs: u64, result: *mut u64, out: *mut VmcallOut) -> u64 {
    // SAFETY: generated code passes the `JIT`-owned blocks embedded at
    // codegen time; `run` does not return until the generated code finishes,
    // so both outlive the call.
    unsafe {
        let out = &mut *out;
        if rhs == 0 {
            out.status = VmcallOut::DIVISION_BY_ZERO;
            return VmcallOut::DIVISION_BY_ZERO;
        }
        *result = (lhs as i64).wrapping_div(rhs as i64) as u64;
        out.status = VmcallOut::OK;
        VmcallOut::OK
    }
}

/// Computes a 64-bit unsigned division exactly like the interpreter
/// (`lhs / rhs`), with a zero-divisor check. See [`jit_sdiv64`].
extern "C" fn jit_udiv64(lhs: u64, rhs: u64, result: *mut u64, out: *mut VmcallOut) -> u64 {
    // SAFETY: see [`jit_sdiv64`].
    unsafe {
        let out = &mut *out;
        if rhs == 0 {
            out.status = VmcallOut::DIVISION_BY_ZERO;
            return VmcallOut::DIVISION_BY_ZERO;
        }
        *result = lhs / rhs;
        out.status = VmcallOut::OK;
        VmcallOut::OK
    }
}

/// Computes a 64-bit signed remainder exactly like the interpreter
/// (`(lhs as i64).wrapping_rem(rhs as i64)`), with a zero-divisor check.
/// See [`jit_sdiv64`].
extern "C" fn jit_srem64(lhs: u64, rhs: u64, result: *mut u64, out: *mut VmcallOut) -> u64 {
    // SAFETY: see [`jit_sdiv64`].
    unsafe {
        let out = &mut *out;
        if rhs == 0 {
            out.status = VmcallOut::DIVISION_BY_ZERO;
            return VmcallOut::DIVISION_BY_ZERO;
        }
        *result = (lhs as i64).wrapping_rem(rhs as i64) as u64;
        out.status = VmcallOut::OK;
        VmcallOut::OK
    }
}

/// Computes a 64-bit unsigned remainder exactly like the interpreter
/// (`lhs % rhs`), with a zero-divisor check. See [`jit_sdiv64`].
extern "C" fn jit_urem64(lhs: u64, rhs: u64, result: *mut u64, out: *mut VmcallOut) -> u64 {
    // SAFETY: see [`jit_sdiv64`].
    unsafe {
        let out = &mut *out;
        if rhs == 0 {
            out.status = VmcallOut::DIVISION_BY_ZERO;
            return VmcallOut::DIVISION_BY_ZERO;
        }
        *result = lhs % rhs;
        out.status = VmcallOut::OK;
        VmcallOut::OK
    }
}

// Short aliases for the `Jcc rel32` (two-byte opcode + 4-byte displacement)
// encodings used for branches inside a single W8 instruction lowering.
const JB: [u8; 2] = [0x0F, 0x82];
const JAE: [u8; 2] = [0x0F, 0x83];
const JE: [u8; 2] = [0x0F, 0x84];
const JNE: [u8; 2] = [0x0F, 0x85];
const JBE: [u8; 2] = [0x0F, 0x86];
const JA: [u8; 2] = [0x0F, 0x87];
const JL: [u8; 2] = [0x0F, 0x8C];
const JG: [u8; 2] = [0x0F, 0x8F];

impl JITCodegen {
    pub fn generate(
        &mut self,
        instructions: &[Instruction],
        ctx: &GenContext,
    ) -> Result<GeneratedCode, VMError> {
        // Pre-scan: dynamic control flow needs the address table after the
        // code; `RET` additionally needs the error stub. Any `RET`,
        // `VMCALL`, or 64-bit division means execution can report into the
        // out block.
        let need_error_stub = instructions
            .iter()
            .any(|i| matches!(i.opcode, OperationCode::RET));
        let need_table = need_error_stub || instructions.iter().any(has_register_target);
        let has_vmcall = instructions
            .iter()
            .any(|i| matches!(i.opcode, OperationCode::VMCALL));
        let has_div = instructions.iter().any(|i| {
            matches!(
                i.opcode,
                OperationCode::SDIV
                    | OperationCode::UDIV
                    | OperationCode::SREM
                    | OperationCode::UREM
            )
        });
        let has_status = need_error_stub || has_vmcall || has_div;

        // Pass 1: emit machine code, remembering each instruction's offset
        // and collecting patches for pass 2.
        let mut machine_code = Vec::new();
        let mut offsets: Vec<usize> = Vec::with_capacity(instructions.len());
        let mut fixups: Vec<Fixup> = Vec::new();

        // Prologue: save the callee-saved registers the generated code
        // uses as scratch (`EBX`, `ESI`, `EDI`) and keep the stack 16-byte
        // aligned for trampoline calls. The i386 System V ABI guarantees
        // `(ESP + 4) % 16 == 0` at the entry point, i.e. `ESP % 16 == 12`;
        // three pushes bring it to 0, so the reserved block must be a
        // multiple of 16 to preserve it (`sub esp, 8` would leave every
        // later `call` misaligned by 8).
        //
        // ```
        // push ebx    ; 53
        // push esi    ; 56
        // push edi    ; 57
        // sub esp, 16 ; 83 EC 10
        // ```
        machine_code.extend_from_slice(&[0x53, 0x56, 0x57, 0x83, 0xEC, 0x10]);

        for (index, instruction) in instructions.iter().enumerate() {
            offsets.push(machine_code.len());

            // Most opcodes produce self-contained bytes; control flow appends
            // directly because it also records fixups.
            let code = match instruction.opcode {
                OperationCode::NOP => self.nop()?,
                OperationCode::MOVE => self.mov(instruction)?,
                OperationCode::IADD => self.iadd(instruction)?,
                OperationCode::ISUB => self.isub(instruction)?,
                OperationCode::IMUL => self.imul(instruction)?,
                OperationCode::SDIV => {
                    self.div_(
                        instruction,
                        jit_sdiv64 as *const () as usize as u64,
                        &mut machine_code,
                        &mut fixups,
                        ctx,
                    )?;
                    continue;
                }
                OperationCode::UDIV => {
                    self.div_(
                        instruction,
                        jit_udiv64 as *const () as usize as u64,
                        &mut machine_code,
                        &mut fixups,
                        ctx,
                    )?;
                    continue;
                }
                OperationCode::SREM => {
                    self.div_(
                        instruction,
                        jit_srem64 as *const () as usize as u64,
                        &mut machine_code,
                        &mut fixups,
                        ctx,
                    )?;
                    continue;
                }
                OperationCode::UREM => {
                    self.div_(
                        instruction,
                        jit_urem64 as *const () as usize as u64,
                        &mut machine_code,
                        &mut fixups,
                        ctx,
                    )?;
                    continue;
                }
                OperationCode::INEG => self.ineg(instruction)?,
                OperationCode::AND => self.and_(instruction)?,
                OperationCode::OR => self.or_(instruction)?,
                OperationCode::XOR => self.xor_(instruction)?,
                OperationCode::NOT => self.not_(instruction)?,
                OperationCode::LSL => self.lsl(instruction)?,
                OperationCode::LSR => self.lsr(instruction)?,
                OperationCode::SAR => self.sar(instruction)?,
                OperationCode::IEQ => self.ieq(instruction)?,
                OperationCode::INE => self.ine(instruction)?,
                OperationCode::SLT => self.slt(instruction)?,
                OperationCode::SLE => self.sle(instruction)?,
                OperationCode::SGT => self.sgt(instruction)?,
                OperationCode::SGE => self.sge(instruction)?,
                OperationCode::ULT => self.ult(instruction)?,
                OperationCode::ULE => self.ule(instruction)?,
                OperationCode::UGT => self.ugt(instruction)?,
                OperationCode::UGE => self.uge(instruction)?,
                OperationCode::FADD => self.fadd(instruction)?,
                OperationCode::FSUB => self.fsub(instruction)?,
                OperationCode::FMUL => self.fmul(instruction)?,
                OperationCode::FDIV => self.fdiv(instruction)?,
                OperationCode::FREM => self.frem(instruction)?,
                OperationCode::FNEG => self.fneg(instruction)?,
                OperationCode::FEQ => self.feq(instruction)?,
                OperationCode::FNE => self.fne(instruction)?,
                OperationCode::FLT => self.flt(instruction)?,
                OperationCode::FLE => self.fle(instruction)?,
                OperationCode::FGT => self.fgt(instruction)?,
                OperationCode::FGE => self.fge(instruction)?,
                OperationCode::LOAD8
                | OperationCode::LOAD16
                | OperationCode::LOAD32
                | OperationCode::LOAD64 => self.load_(instruction, ctx)?,
                OperationCode::STORE8
                | OperationCode::STORE16
                | OperationCode::STORE32
                | OperationCode::STORE64 => self.store_(instruction, ctx)?,
                OperationCode::JMP => {
                    self.jmp_(instruction, &mut machine_code, &mut fixups)?;
                    continue;
                }
                OperationCode::JZ => {
                    self.jz_(instruction, &mut machine_code, &mut fixups)?;
                    continue;
                }
                OperationCode::JNZ => {
                    self.jnz_(instruction, &mut machine_code, &mut fixups)?;
                    continue;
                }
                OperationCode::CALL => {
                    self.call_(instruction, index, &mut machine_code, &mut fixups)?;
                    continue;
                }
                OperationCode::RET => {
                    self.ret_(instruction, &mut machine_code, &mut fixups, ctx)?;
                    continue;
                }
                OperationCode::VMCALL => {
                    self.vmcall(instruction, &mut machine_code, &mut fixups, ctx)?;
                    continue;
                }
            };

            machine_code.extend(code);
        }

        // Error stub: `RET` on an empty call stack reports the failure and
        // falls through to the epilogue. The out block address is known
        // at emission time (`status` is its first field).
        let error_stub = if need_error_stub {
            let stub = machine_code.len();
            let status = addr32(ctx.out);
            let status_hi = status
                .checked_add(4)
                .expect("out block address overflows the x86 address space");

            // `status` is a `u64`, so both halves are written; a lone 32-bit
            // store would leave the upper half at whatever it held before.
            //
            // `mov dword [<out.status>], EMPTY_CALL_STACK`
            machine_code.extend_from_slice(&[0xC7, 0x05]);
            machine_code.extend_from_slice(&status.to_le_bytes());
            machine_code.extend_from_slice(
                &u32::try_from(VmcallOut::EMPTY_CALL_STACK)
                    .expect("status code does not fit in 32 bits")
                    .to_le_bytes(),
            );
            // `mov dword [<out.status + 4>], 0`
            machine_code.extend_from_slice(&[0xC7, 0x05]);
            machine_code.extend_from_slice(&status_hi.to_le_bytes());
            machine_code.extend_from_slice(&0u32.to_le_bytes());

            Some(stub)
        } else {
            None
        };

        // Return control to the caller, restoring the callee-saved
        // registers first. Out-of-range jumps land here, mirroring the
        // interpreter (`next >= instruction_count` ends execution).
        //
        // ```
        // add esp, 16 ; 83 C4 10
        // pop edi     ; 5F
        // pop esi     ; 5E
        // pop ebx     ; 5B
        // ret         ; C3
        // ```
        let epilogue = machine_code.len();
        machine_code.extend_from_slice(&[0x83, 0xC4, 0x10, 0x5F, 0x5E, 0x5B, 0xC3]);

        // The address table follows the epilogue, 4-byte aligned
        // (padding is never executed). Status cells are NOT placed
        // here: the mapping becomes read-execute before execution, so
        // everything the generated code writes lives in `ctx` instead (W^X).
        let table_base = if need_table {
            while machine_code.len() % 4 != 0 {
                machine_code.push(0x90); //< `nop`
            }
            let table_base = ctx.code_base + machine_code.len() as u64;
            for &offset in &offsets {
                machine_code
                    .extend_from_slice(&addr32(ctx.code_base + offset as u64).to_le_bytes());
            }
            Some(table_base)
        } else {
            None
        };

        // Pass 2: resolve displacements and absolute addresses.
        for fixup in &fixups {
            match *fixup {
                Fixup::RelToInstr { pos, target } => {
                    let target_off = if target < offsets.len() {
                        offsets[target]
                    } else {
                        epilogue
                    };
                    patch_rel32(&mut machine_code, pos, target_off);
                }
                Fixup::RelToEpilogue { pos } => patch_rel32(&mut machine_code, pos, epilogue),
                Fixup::RelToErrorStub { pos } => patch_rel32(
                    &mut machine_code,
                    pos,
                    error_stub.expect("`RET` fixup without an error stub"),
                ),
                Fixup::TableBase { pos } => machine_code[pos..pos + 4].copy_from_slice(
                    &addr32(table_base.expect("table fixup without a table")).to_le_bytes(),
                ),
                Fixup::TableLen { pos } => machine_code[pos..pos + 4]
                    .copy_from_slice(&(offsets.len() as u32).to_le_bytes()),
            }
        }

        Ok(GeneratedCode {
            bytes: machine_code,
            has_status,
        })
    }

    fn mov(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let operands = (
            instruction.operand1,
            instruction.operand2,
            instruction.operand3,
        );

        let mut code = Vec::new();

        match operands {
            (Some(op1), Some(op2), None) => {
                let destination = op1.expect_register()?;
                self.load_eax_edx(&mut code, op2.kind);
                self.store_eax_edx(&mut code, destination);
                Ok(code)
            }

            // `MOVE` requires exactly 2 operands.
            _ => {
                let got = [operands.0, operands.1, operands.2]
                    .into_iter()
                    .filter(Option::is_some)
                    .count() as u8;

                Err(VMError::new(VMErrorKind::IncorrectNumberOfOperands {
                    expected: 2,
                    got,
                }))
            }
        }
    }

    fn iadd(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `add eax, ecx`
            code.extend_from_slice(&[0x01, 0xC8]);
            // `adc edx, ebx` — the high halves plus the carry of the low add.
            code.extend_from_slice(&[0x11, 0xDA]);
        })
    }

    fn isub(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `sub eax, ecx`
            code.extend_from_slice(&[0x29, 0xC8]);
            // `sbb edx, ebx` — the high halves minus the borrow of the low sub.
            //
            // `19 /r` is `SBB r/m32, r32` (the ModRM `rm` field is the
            // destination); `1B /r` would assemble to `sbb ebx, edx`.
            code.extend_from_slice(&[0x19, 0xDA]);
        })
    }

    fn imul(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_integer_operands(&mut code, lhs.kind, rhs.kind);

        // Only the low 64 bits of the full 128-bit product are kept
        // (`wrapping_mul`): `a_lo * b_lo + 2^32 * (a_lo * b_hi + a_hi * b_lo)`.
        // The operands are spilled to the stack first because `mul` destroys
        // `EDX:EAX`.
        //
        // `sub esp, 16`
        code.extend_from_slice(&[0x83, 0xEC, 0x10]);
        // `mov [esp], eax` (a_lo), `mov [esp + 4], edx` (a_hi),
        // `mov [esp + 8], ecx` (b_lo), `mov [esp + 12], ebx` (b_hi)
        code.extend_from_slice(&[0x89, 0x04, 0x24]);
        code.extend_from_slice(&[0x89, 0x54, 0x24, 0x04]);
        code.extend_from_slice(&[0x89, 0x4C, 0x24, 0x08]);
        code.extend_from_slice(&[0x89, 0x5C, 0x24, 0x0C]);
        // `mov eax, [esp]` (a_lo); `mul dword [esp + 8]` (b_lo):
        // `EDX:EAX = a_lo * b_lo`
        code.extend_from_slice(&[0x8B, 0x04, 0x24]);
        code.extend_from_slice(&[0xF7, 0x64, 0x24, 0x08]);
        // `mov ecx, eax` (result low); `mov ebx, edx` (partial high)
        code.extend_from_slice(&[0x89, 0xC1]);
        code.extend_from_slice(&[0x89, 0xD3]);
        // `mov eax, [esp]` (a_lo); `mul dword [esp + 12]` (b_hi);
        // `add ebx, eax` (accumulate the low 32 bits of `a_lo * b_hi`)
        code.extend_from_slice(&[0x8B, 0x04, 0x24]);
        code.extend_from_slice(&[0xF7, 0x64, 0x24, 0x0C]);
        code.extend_from_slice(&[0x01, 0xC3]);
        // `mov eax, [esp + 4]` (a_hi); `mul dword [esp + 8]` (b_lo);
        // `add ebx, eax` (accumulate the low 32 bits of `a_hi * b_lo`)
        code.extend_from_slice(&[0x8B, 0x44, 0x24, 0x04]);
        code.extend_from_slice(&[0xF7, 0x64, 0x24, 0x08]);
        code.extend_from_slice(&[0x01, 0xC3]);
        // `mov eax, ecx` (result low); `mov edx, ebx` (result high)
        code.extend_from_slice(&[0x89, 0xC8]);
        code.extend_from_slice(&[0x89, 0xDA]);
        // `add esp, 16`
        code.extend_from_slice(&[0x83, 0xC4, 0x10]);

        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    /// Lowers `SDIV`, `UDIV`, `SREM`, and `UREM` through the matching
    /// 64-bit helper (see [`jit_sdiv64`]): the operands are passed as two
    /// `u64` stack arguments, the helper writes the result to a temporary
    /// cell in the same frame, and a nonzero outcome branches to the
    /// epilogue.
    fn div_(
        &self,
        instruction: &Instruction,
        trampoline: u64,
        code: &mut Vec<u8>,
        fixups: &mut Vec<Fixup>,
        ctx: &GenContext,
    ) -> Result<(), VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;

        // `sub esp, 32` — room for 6 argument slots plus the 8-byte result
        // cell at `[esp + 24]`; keeps the 16-byte alignment for the call.
        code.extend_from_slice(&[0x83, 0xEC, 0x20]);

        // `lhs` as the first `u64` argument at `[esp]` or `[esp + 4]`.
        let mut halves = Vec::new();
        self.load_eax_edx(&mut halves, lhs.kind);
        code.extend(halves);
        // `mov [esp], eax`
        code.extend_from_slice(&[0x89, 0x04, 0x24]);
        // `mov [esp + 4], edx`
        code.extend_from_slice(&[0x89, 0x54, 0x24, 0x04]);

        // `rhs` as the second `u64` argument at `[esp + 8]` or `[esp + 12]`.
        let mut halves = Vec::new();
        self.load_eax_edx(&mut halves, rhs.kind);
        code.extend(halves);
        // `mov [esp + 8], eax`
        code.extend_from_slice(&[0x89, 0x44, 0x24, 0x08]);
        // `mov [esp + 12], edx`
        code.extend_from_slice(&[0x89, 0x54, 0x24, 0x0C]);

        // The result pointer (`*mut u64`) at `[esp + 16]` points at the
        // temporary cell `[esp + 24]`.
        // `mov eax, esp`
        code.extend_from_slice(&[0x89, 0xE0]);
        // `add eax, 24`
        code.extend_from_slice(&[0x83, 0xC0, 0x18]);
        // `mov [esp + 16], eax`
        code.extend_from_slice(&[0x89, 0x44, 0x24, 0x10]);

        // The out-block pointer at `[esp + 20]`.
        // `mov eax, <out>`
        code.extend_from_slice(&[0xB8]);
        code.extend_from_slice(&addr32(ctx.out).to_le_bytes());
        // `mov [esp + 20], eax`
        code.extend_from_slice(&[0x89, 0x44, 0x24, 0x14]);

        // `mov eax, <trampoline>`
        code.extend_from_slice(&[0xB8]);
        code.extend_from_slice(&addr32(trampoline).to_le_bytes());

        // `call eax`
        code.extend_from_slice(&[0xFF, 0xD0]);

        // Save the outcome code: the result cell below is released before
        // any branch, so the stack is balanced on every path. The helper
        // returns a `u64` in EDX:EAX, so both halves are folded in — the
        // same test `vmcall` performs.
        // `mov ecx, eax`
        code.extend_from_slice(&[0x89, 0xC1]);
        // `or ecx, edx`
        code.extend_from_slice(&[0x09, 0xD1]);

        // `mov eax, [esp + 24]`
        code.extend_from_slice(&[0x8B, 0x44, 0x24, 0x18]);
        // `mov edx, [esp + 28]`
        code.extend_from_slice(&[0x8B, 0x54, 0x24, 0x1C]);
        // `add esp, 32`
        code.extend_from_slice(&[0x83, 0xC4, 0x20]);

        // A nonzero outcome (division by zero) exits through the epilogue.
        // `test ecx, ecx`
        code.extend_from_slice(&[0x85, 0xC9]);

        // `jnz <epilogue>`
        fixups.push(Fixup::RelToEpilogue {
            pos: code.len() + 2,
        });
        code.extend_from_slice(&[0x0F, 0x85, 0, 0, 0, 0]);

        let mut tail = Vec::new();
        self.store_eax_edx(&mut tail, destination);
        code.extend(tail);

        Ok(())
    }

    fn ineg(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, source) = instruction.expect2()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_eax_edx(&mut code, source.kind);

        // Two's-complement negation of a 64-bit value through the carry:
        // `neg eax` complements the low half and sets CF unless it was
        // zero; `adc edx, 0` propagates the carry; `neg edx` complements
        // the high half.
        // `neg eax`
        code.extend_from_slice(&[0xF7, 0xD8]);
        // `adc edx, 0`
        code.extend_from_slice(&[0x83, 0xD2, 0x00]);
        // `neg edx`
        code.extend_from_slice(&[0xF7, 0xDA]);

        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn and_(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `and eax, ecx`
            code.extend_from_slice(&[0x21, 0xC8]);
            // `and edx, ebx`
            code.extend_from_slice(&[0x21, 0xDA]);
        })
    }

    fn or_(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `or eax, ecx`
            code.extend_from_slice(&[0x09, 0xC8]);
            // `or edx, ebx`
            code.extend_from_slice(&[0x09, 0xDA]);
        })
    }

    fn xor_(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `xor eax, ecx`
            code.extend_from_slice(&[0x31, 0xC8]);
            // `xor edx, ebx`
            code.extend_from_slice(&[0x31, 0xDA]);
        })
    }

    fn not_(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, source) = instruction.expect2()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_eax_edx(&mut code, source.kind);

        // Bitwise inversion of both halves.
        // `not eax`
        code.extend_from_slice(&[0xF7, 0xD0]);
        // `not edx`
        code.extend_from_slice(&[0xF7, 0xD2]);

        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn lsl(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_shift_operands(&mut code, lhs.kind, rhs.kind);

        // The 64-bit shift count is masked to 6 bits, matching
        // `wrapping_shl` in the interpreter (only the low 32 bits of the
        // count are used, like `r as u32`).
        // `and ecx, 63`
        code.extend_from_slice(&[0x83, 0xE1, 0x3F]);
        // `cmp ecx, 32`
        code.extend_from_slice(&[0x83, 0xF9, 0x20]);
        // `jb <lo32>`
        let lo32 = Self::emit_jcc(&mut code, JB);
        // `count >= 32`: the low half shifts out entirely.
        // `sub ecx, 32`
        code.extend_from_slice(&[0x83, 0xE9, 0x20]);
        // `mov edx, eax`
        code.extend_from_slice(&[0x89, 0xC2]);
        // `xor eax, eax`
        code.extend_from_slice(&[0x31, 0xC0]);
        // `shl edx, cl`
        code.extend_from_slice(&[0xD3, 0xE2]);
        // `jmp <done>`
        let done_from_hi = Self::emit_jmp(&mut code);
        // `<lo32>: shld edx, eax, cl` (the high half gains the top bits of
        // the low half).
        let lo32_off = code.len();
        patch_rel32(&mut code, lo32, lo32_off);
        code.extend_from_slice(&[0x0F, 0xA5, 0xC2]);
        // `shl eax, cl`
        code.extend_from_slice(&[0xD3, 0xE0]);
        // `<done>:`
        let done_off = code.len();
        patch_rel32(&mut code, done_from_hi, done_off);

        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn lsr(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_shift_operands(&mut code, lhs.kind, rhs.kind);

        // `and ecx, 63`
        code.extend_from_slice(&[0x83, 0xE1, 0x3F]);
        // `cmp ecx, 32`
        code.extend_from_slice(&[0x83, 0xF9, 0x20]);
        // `jb <lo32>`
        let lo32 = Self::emit_jcc(&mut code, JB);
        // `count >= 32`: the high half shifts down into the low half.
        // `sub ecx, 32`
        code.extend_from_slice(&[0x83, 0xE9, 0x20]);
        // `mov eax, edx`
        code.extend_from_slice(&[0x89, 0xD0]);
        // `xor edx, edx`
        code.extend_from_slice(&[0x31, 0xD2]);
        // `shr eax, cl`
        code.extend_from_slice(&[0xD3, 0xE8]);
        // `jmp <done>`
        let done_from_hi = Self::emit_jmp(&mut code);
        // `<lo32>: shrd eax, edx, cl` (the low half gains the bottom bits
        // of the high half).
        let lo32_off = code.len();
        patch_rel32(&mut code, lo32, lo32_off);
        code.extend_from_slice(&[0x0F, 0xAD, 0xD0]);
        // `shr edx, cl`
        code.extend_from_slice(&[0xD3, 0xEA]);
        // `<done>:`
        let done_off = code.len();
        patch_rel32(&mut code, done_from_hi, done_off);

        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn sar(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_shift_operands(&mut code, lhs.kind, rhs.kind);

        // `and ecx, 63`
        code.extend_from_slice(&[0x83, 0xE1, 0x3F]);
        // `cmp ecx, 32`
        code.extend_from_slice(&[0x83, 0xF9, 0x20]);
        // `jb <lo32>`
        let lo32 = Self::emit_jcc(&mut code, JB);
        // `count >= 32`: the sign-extended high half shifts down.
        // `sub ecx, 32`
        code.extend_from_slice(&[0x83, 0xE9, 0x20]);
        // `mov eax, edx`
        code.extend_from_slice(&[0x89, 0xD0]);
        // `sar eax, cl`
        code.extend_from_slice(&[0xD3, 0xF8]);
        // `sar edx, 31` (the high half becomes the sign bit spread).
        code.extend_from_slice(&[0xC1, 0xFA, 0x1F]);
        // `jmp <done>`
        let done_from_hi = Self::emit_jmp(&mut code);
        // `<lo32>: shrd eax, edx, cl` (the low half gains the bottom bits
        // of the high half, which keeps its sign).
        let lo32_off = code.len();
        patch_rel32(&mut code, lo32, lo32_off);
        code.extend_from_slice(&[0x0F, 0xAD, 0xD0]);
        // `sar edx, cl`
        code.extend_from_slice(&[0xD3, 0xFA]);
        // `<done>:`
        let done_off = code.len();
        patch_rel32(&mut code, done_from_hi, done_off);

        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn ieq(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.eq_ne(instruction, true)
    }

    fn ine(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.eq_ne(instruction, false)
    }

    fn slt(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Signed less-than: the high halves decide signed, the low halves
        // unsigned.
        self.ordered(instruction, JL, JG, JB)
    }

    fn sle(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Signed less-than or equal.
        self.ordered(instruction, JL, JG, JBE)
    }

    fn sgt(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Signed greater-than.
        self.ordered(instruction, JG, JL, JA)
    }

    fn sge(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Signed greater-than or equal.
        self.ordered(instruction, JG, JL, JAE)
    }

    fn ult(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Unsigned less-than (both halves unsigned).
        self.ordered(instruction, JB, JA, JB)
    }

    fn ule(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Unsigned less-than or equal.
        self.ordered(instruction, JB, JA, JBE)
    }

    fn ugt(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Unsigned greater-than.
        self.ordered(instruction, JA, JB, JA)
    }

    fn uge(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Unsigned greater-than or equal.
        self.ordered(instruction, JA, JB, JAE)
    }

    fn fadd(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_float(instruction, |code| {
            // Add the scalar double-precision values in XMM0 and XMM1.
            // `addsd xmm0, xmm1`
            code.extend_from_slice(&[0xF2, 0x0F, 0x58, 0xC1]);
        })
    }

    fn fsub(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_float(instruction, |code| {
            // Subtract the scalar double-precision value in XMM1 from XMM0.
            // `subsd xmm0, xmm1`
            code.extend_from_slice(&[0xF2, 0x0F, 0x5C, 0xC1]);
        })
    }

    fn fmul(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_float(instruction, |code| {
            // Multiply the scalar double-precision values in XMM0 and XMM1.
            // `mulsd xmm0, xmm1`
            code.extend_from_slice(&[0xF2, 0x0F, 0x59, 0xC1]);
        })
    }

    fn fdiv(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_float(instruction, |code| {
            // Divide the scalar double-precision value in XMM0 by XMM1.
            // `divsd xmm0, xmm1`
            code.extend_from_slice(&[0xF2, 0x0F, 0x5E, 0xC1]);
        })
    }

    fn frem(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_integer_operands(&mut code, lhs.kind, rhs.kind);
        // Save both operands on the stack for the x87 remainder instruction.
        // `sub esp, 16`
        code.extend_from_slice(&[0x83, 0xEC, 0x10]);
        // `mov [esp], eax`
        code.extend_from_slice(&[0x89, 0x04, 0x24]);
        // `mov [esp + 4], edx`
        code.extend_from_slice(&[0x89, 0x54, 0x24, 0x04]);
        // `mov [esp + 8], ecx`
        code.extend_from_slice(&[0x89, 0x4C, 0x24, 0x08]);
        // `mov [esp + 12], ebx`
        code.extend_from_slice(&[0x89, 0x5C, 0x24, 0x0C]);

        // Load rhs first and lhs second so FPREM computes ST0 % ST1 as lhs % rhs.
        // `fld qword [esp + 8]`
        code.extend_from_slice(&[0xDD, 0x44, 0x24, 0x08]);
        // `fld qword [esp]`
        code.extend_from_slice(&[0xDD, 0x04, 0x24]);

        // FPREM can be partial, so repeat while the x87 C2 status bit is set.
        // `fprem`
        // `DE F8`
        let fprem_offset = code.len();
        code.extend_from_slice(&[0xDE, 0xF8]);
        // `fnstsw ax`
        code.extend_from_slice(&[0xDF, 0xE0]);
        // `test ah, 04h`
        code.extend_from_slice(&[0xF6, 0xC4, 0x04]);
        // `jnz fprem`
        // `75 <relative displacement>`
        let displacement = fprem_offset as isize - (code.len() as isize + 2);
        code.extend_from_slice(&[0x75, displacement as i8 as u8]);

        // Store the remainder from ST0 and pop it from the x87 stack.
        // `fstp qword [esp]`
        code.extend_from_slice(&[0xDD, 0x1C, 0x24]);
        // Discard the divisor left in ST0.
        // `fstp st0`
        code.extend_from_slice(&[0xDD, 0xD8]);
        // `mov eax, [esp]`
        code.extend_from_slice(&[0x8B, 0x04, 0x24]);
        // `mov edx, [esp + 4]`
        code.extend_from_slice(&[0x8B, 0x54, 0x24, 0x04]);
        // `add esp, 16`
        code.extend_from_slice(&[0x83, 0xC4, 0x10]);

        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn fneg(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, source) = instruction.expect2()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_eax_edx(&mut code, source.kind);

        // Flip only the sign bit to negate the f64 value.
        // `xor edx, 80000000h`
        code.extend_from_slice(&[0x81, 0xF2, 0x00, 0x00, 0x00, 0x80]);

        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn feq(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Ordered equality: ZF is also set when unordered (NaN), so the
        // parity flag (set by `ucomisd` on unordered) must be excluded.
        //
        // ```x86asm
        // sete al    ; 0F 94 C0
        // setnp cl   ; 0F 9B C1
        // and al, cl ; 22 C1
        // ```
        self.binary_fcompare(
            instruction,
            &[0x0F, 0x94, 0xC0, 0x0F, 0x9B, 0xC1, 0x22, 0xC1],
        )
    }

    fn fne(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Ordered inequality OR unordered (NaN): Rust `!=` is true for NaN.
        //
        // ```x86asm
        // setne al ; 0F 95 C0
        // setp cl  ; 0F 9A C1
        // or al, cl ; 0A C1
        // ```
        self.binary_fcompare(
            instruction,
            &[0x0F, 0x95, 0xC0, 0x0F, 0x9A, 0xC1, 0x0A, 0xC1],
        )
    }

    fn flt(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Ordered less-than: CF is also set when unordered, so exclude it
        // via the parity flag.
        //
        // ```x86asm
        // setb al    ; 0F 92 C0
        // setnp cl   ; 0F 9B C1
        // and al, cl ; 22 C1
        // ```
        self.binary_fcompare(
            instruction,
            &[0x0F, 0x92, 0xC0, 0x0F, 0x9B, 0xC1, 0x22, 0xC1],
        )
    }

    fn fle(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // ```x86asm
        // setbe al   ; 0F 96 C0
        // setnp cl   ; 0F 9B C1
        // and al, cl ; 22 C1
        // ```
        self.binary_fcompare(
            instruction,
            &[0x0F, 0x96, 0xC0, 0x0F, 0x9B, 0xC1, 0x22, 0xC1],
        )
    }

    fn fgt(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Ordered greater-than: unordered sets ZF, so `seta` (ZF=0 and CF=0)
        // already excludes NaN without a parity check.
        //
        // `seta al`
        // `0F 97 C0`
        self.binary_fcompare(instruction, &[0x0F, 0x97, 0xC0])
    }

    fn fge(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Ordered greater-than or equal: unordered sets CF, so `setae`
        // (CF=0) already excludes NaN without a parity check.
        //
        // `setae al`
        // `0F 93 C0`
        self.binary_fcompare(instruction, &[0x0F, 0x93, 0xC0])
    }

    fn nop(&self) -> Result<Vec<u8>, VMError> {
        Ok(vec![0x90]) //< `nop`
    }

    fn load_(&self, instruction: &Instruction, ctx: &GenContext) -> Result<Vec<u8>, VMError> {
        let (destination, source) = instruction.expect2()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        // Load the VM memory address (offset) into EAX. Only the low 32
        // bits are used, matching `as usize` truncation in the interpreter
        // on 32-bit targets.
        match source.kind {
            // `mov eax, [absolute address]`
            // `A1 <addr32>`
            OperandKind::Register(register) => {
                self.mov_eax_register(&mut code, register.0 as usize)
            }

            // `mov eax, <immediate low 32 bits>`
            // `B8 <imm32>`
            OperandKind::Immediate(value) => self.mov_eax_immediate(&mut code, value as u32),
        }

        // Load the current base pointer of the VM memory into EDX.
        //
        // The base lives in a cell (refreshed by the `VMCALL` trampoline
        // after every host call), never baked in: a host may replace the
        // buffer mid-run.
        //
        // `mov edx, <membase cell>` — the *address* of the cell, as an
        // immediate (`BA <imm32>`); `8B 15 <disp32>` would already load the
        // cell's contents and the dereference below would read the guest
        // memory instead of using it as the base.
        code.extend_from_slice(&[0xBA]);
        code.extend_from_slice(&addr32(ctx.membase_addr).to_le_bytes());
        // `mov edx, [edx]`
        code.extend_from_slice(&[0x8B, 0x12]);

        // Add the requested address to the base pointer.
        //
        // `add edx, eax`
        //
        // After this EDX points at the first byte to read.
        code.extend_from_slice(&[0x01, 0xC2]);

        // Load from `[edx]` with zero-extension into EAX:EDX.
        match instruction.opcode {
            // `movzx eax, byte [edx]`; the high half is zero.
            OperationCode::LOAD8 => {
                code.extend_from_slice(&[0x0F, 0xB6, 0x02]);
                // `xor edx, edx`
                code.extend_from_slice(&[0x31, 0xD2]);
            }

            // `movzx eax, word [edx]`; the high half is zero.
            OperationCode::LOAD16 => {
                code.extend_from_slice(&[0x0F, 0xB7, 0x02]);
                // `xor edx, edx`
                code.extend_from_slice(&[0x31, 0xD2]);
            }

            // `mov eax, dword [edx]`; the high half is zero.
            OperationCode::LOAD32 => {
                code.extend_from_slice(&[0x8B, 0x02]);
                // `xor edx, edx`
                code.extend_from_slice(&[0x31, 0xD2]);
            }

            // `mov eax, dword [edx]`; `mov edx, dword [edx + 4]`
            OperationCode::LOAD64 => {
                code.extend_from_slice(&[0x8B, 0x02]);
                code.extend_from_slice(&[0x8B, 0x52, 0x04]);
            }

            _ => unimplemented!(),
        }

        // Store the zero-extended value into the destination register.
        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn store_(&self, instruction: &Instruction, ctx: &GenContext) -> Result<Vec<u8>, VMError> {
        let (address, value) = instruction.expect2()?;
        let mut code = Vec::new();

        // Load the VM memory address (offset) into EAX (low 32 bits, like
        // `LOAD*`).
        match address.kind {
            // `mov eax, [absolute address]`
            OperandKind::Register(register) => {
                self.mov_eax_register(&mut code, register.0 as usize)
            }

            // `mov eax, <immediate low 32 bits>`
            OperandKind::Immediate(value) => self.mov_eax_immediate(&mut code, value as u32),
        }

        // Load the value to store into ECX (low) and EBX (high).
        //
        // Only the low 8, 16, 32, or 64 bits are written, matching the
        // interpreter truncation.
        match value.kind {
            OperandKind::Register(register) => {
                self.mov_ecx_register(&mut code, register.0 as usize);
                // The high half lives 4 bytes past the cell start; reading
                // the cell start twice would store `lo:lo` on `STORE64`.
                self.mov_ebx_register_hi(&mut code, register.0 as usize);
            }
            OperandKind::Immediate(value) => {
                self.mov_ecx_immediate(&mut code, value as u32);
                self.mov_ebx_immediate(&mut code, (value >> 32) as u32);
            }
        }

        // Load the current base pointer of the VM memory into EDX
        // (see [`load_`](JITCodegen::load_)).
        //
        // `mov edx, <membase cell>` — the *address* of the cell, as an
        // immediate (`BA <imm32>`); `8B 15 <disp32>` would already load the
        // cell's contents and the dereference below would read the guest
        // memory instead of using it as the base.
        code.extend_from_slice(&[0xBA]);
        code.extend_from_slice(&addr32(ctx.membase_addr).to_le_bytes());
        // `mov edx, [edx]`
        code.extend_from_slice(&[0x8B, 0x12]);

        // Add the requested address to the base pointer.
        //
        // `add edx, eax`
        //
        // After this EDX points at the first byte to write.
        code.extend_from_slice(&[0x01, 0xC2]);

        match instruction.opcode {
            // `mov [edx], cl`
            OperationCode::STORE8 => code.extend_from_slice(&[0x88, 0x0A]),

            // `mov [edx], cx`
            OperationCode::STORE16 => code.extend_from_slice(&[0x66, 0x89, 0x0A]),

            // `mov [edx], ecx`
            OperationCode::STORE32 => code.extend_from_slice(&[0x89, 0x0A]),

            // `mov [edx], ecx`; `mov [edx + 4], ebx`
            OperationCode::STORE64 => {
                code.extend_from_slice(&[0x89, 0x0A]);
                code.extend_from_slice(&[0x89, 0x5A, 0x04]);
            }

            _ => unimplemented!(),
        }

        Ok(code)
    }

    fn jmp_(
        &self,
        instruction: &Instruction,
        code: &mut Vec<u8>,
        fixups: &mut Vec<Fixup>,
    ) -> Result<(), VMError> {
        let target = instruction.expect1()?;

        match target.kind {
            // Direct jump: the displacement is patched in pass 2
            // (out-of-range targets land on the epilogue and exit silently,
            // like the interpreter).
            //
            // `jmp rel32`
            OperandKind::Immediate(value) => {
                fixups.push(Fixup::RelToInstr {
                    pos: code.len() + 1,
                    target: value as usize,
                });
                code.extend_from_slice(&[0xE9, 0, 0, 0, 0]);
            }

            // The target lives in a register, so it is known only at runtime:
            // indirect jump through the address table.
            OperandKind::Register(register) => {
                self.mov_eax_register(code, register.0 as usize);
                self.emit_dynamic_jump(code, fixups);
            }
        }

        Ok(())
    }

    fn jz_(
        &self,
        instruction: &Instruction,
        code: &mut Vec<u8>,
        fixups: &mut Vec<Fixup>,
    ) -> Result<(), VMError> {
        let (condition, target) = instruction.expect2()?;

        match target.kind {
            OperandKind::Immediate(value) => {
                let mut cond = Vec::new();
                self.load_eax_edx(&mut cond, condition.kind);
                code.extend(cond);

                // The full 64-bit condition is zero iff both halves are.
                // `or eax, edx`
                code.extend_from_slice(&[0x09, 0xD0]);

                // `jz rel32`
                fixups.push(Fixup::RelToInstr {
                    pos: code.len() + 2,
                    target: value as usize,
                });
                code.extend_from_slice(&[0x0F, 0x84, 0, 0, 0, 0]);
            }

            OperandKind::Register(register) => {
                // The target index goes to EAX first; the condition follows
                // into ECX:EBX (those loads never touch EAX).
                self.mov_eax_register(code, register.0 as usize);
                let mut cond = Vec::new();
                self.load_ecx_ebx(&mut cond, condition.kind);
                code.extend(cond);

                // Skip the dynamic jump when the condition is nonzero.
                // `or ecx, ebx` — the high half is in EBX (see
                // `load_ecx_ebx`); EDX holds unrelated leftovers here.
                code.extend_from_slice(&[0x09, 0xD9]);

                // `jnz <next>`
                let skip_pos = Self::emit_jcc(code, JNE);
                self.emit_dynamic_jump(code, fixups);
                let next_off = code.len();
                patch_rel32(code, skip_pos, next_off);
            }
        }

        Ok(())
    }

    fn jnz_(
        &self,
        instruction: &Instruction,
        code: &mut Vec<u8>,
        fixups: &mut Vec<Fixup>,
    ) -> Result<(), VMError> {
        let (condition, target) = instruction.expect2()?;

        match target.kind {
            OperandKind::Immediate(value) => {
                let mut cond = Vec::new();
                self.load_eax_edx(&mut cond, condition.kind);
                code.extend(cond);

                // `or eax, edx`
                code.extend_from_slice(&[0x09, 0xD0]);

                // `jnz rel32`
                fixups.push(Fixup::RelToInstr {
                    pos: code.len() + 2,
                    target: value as usize,
                });
                code.extend_from_slice(&[0x0F, 0x85, 0, 0, 0, 0]);
            }

            OperandKind::Register(register) => {
                // Same layout as `jz_` with the skip condition inverted.
                self.mov_eax_register(code, register.0 as usize);
                let mut cond = Vec::new();
                self.load_ecx_ebx(&mut cond, condition.kind);
                code.extend(cond);

                // `or ecx, ebx` — see `jz_`.
                code.extend_from_slice(&[0x09, 0xD9]);

                // Skip the dynamic jump when the condition is zero.
                // `jz <next>`
                let skip_pos = Self::emit_jcc(code, JE);
                self.emit_dynamic_jump(code, fixups);
                let next_off = code.len();
                patch_rel32(code, skip_pos, next_off);
            }
        }

        Ok(())
    }

    fn call_(
        &self,
        instruction: &Instruction,
        index: usize,
        code: &mut Vec<u8>,
        fixups: &mut Vec<Fixup>,
    ) -> Result<(), VMError> {
        let target = instruction.expect1()?;

        // The return address is the following instruction, like the
        // interpreter pushing `ip + 1` — a constant known at codegen time.
        let ret_idx = index + 1;

        match target.kind {
            OperandKind::Immediate(value) => {
                self.emit_call_push(code, ret_idx);

                // `jmp rel32`
                fixups.push(Fixup::RelToInstr {
                    pos: code.len() + 1,
                    target: value as usize,
                });
                code.extend_from_slice(&[0xE9, 0, 0, 0, 0]);
            }

            OperandKind::Register(register) => {
                self.mov_eax_register(code, register.0 as usize);

                // Spill the target index across the trampoline call
                // (`call` leaves EAX unspecified). A `push` or `pop` pair would
                // shift ESP by 4 for the whole nested call, so the slot is
                // taken from a 16-byte block instead and the alignment
                // established by the prologue survives.
                //
                // `sub esp, 16`
                code.extend_from_slice(&[0x83, 0xEC, 0x10]);
                // `mov [esp], eax`
                code.extend_from_slice(&[0x89, 0x04, 0x24]);

                self.emit_call_push(code, ret_idx);

                // `mov eax, [esp]`
                code.extend_from_slice(&[0x8B, 0x04, 0x24]);
                // `add esp, 16`
                code.extend_from_slice(&[0x83, 0xC4, 0x10]);

                self.emit_dynamic_jump(code, fixups);
            }
        }

        Ok(())
    }

    fn ret_(
        &self,
        instruction: &Instruction,
        code: &mut Vec<u8>,
        fixups: &mut Vec<Fixup>,
        ctx: &GenContext,
    ) -> Result<(), VMError> {
        // `RET` takes no operands.
        if instruction.operand_count() != 0 {
            return Err(VMError::new(VMErrorKind::IncorrectNumberOfOperands {
                expected: 0,
                got: instruction.operand_count() as u8,
            }));
        }

        let stack = addr32(self.call_stack as u64);
        let trampoline = addr32(jit_call_pop as *const () as usize as u64);

        // `sub esp, 16` — room for the two arguments, keeps alignment.
        code.extend_from_slice(&[0x83, 0xEC, 0x10]);

        // `mov dword [esp], <stack>`
        code.extend_from_slice(&[0xC7, 0x44, 0x24, 0x00]);
        code.extend_from_slice(&stack.to_le_bytes());

        // `mov eax, <scratch>` — a `JIT`-owned cell, address known upfront.
        code.extend_from_slice(&[0xB8]);
        code.extend_from_slice(&addr32(ctx.scratch_addr).to_le_bytes());
        // `mov [esp + 4], eax`
        code.extend_from_slice(&[0x89, 0x44, 0x24, 0x04]);

        // `mov eax, <trampoline>`
        code.extend_from_slice(&[0xB8]);
        code.extend_from_slice(&trampoline.to_le_bytes());

        // `call eax`
        code.extend_from_slice(&[0xFF, 0xD0]);

        // `add esp, 16`
        code.extend_from_slice(&[0x83, 0xC4, 0x10]);

        // The trampoline returns 1 with the popped index in the scratch
        // cell, or 0 when the stack is empty.
        // `test al, al`
        code.extend_from_slice(&[0x84, 0xC0]);

        // An empty stack reports EMPTY_CALL_STACK through the status cell
        // and returns (mirroring the interpreter's `EmptyCallStack` error).
        // `jz <error stub>`
        fixups.push(Fixup::RelToErrorStub {
            pos: code.len() + 2,
        });
        code.extend_from_slice(&[0x0F, 0x84, 0, 0, 0, 0]);

        // `mov eax, [scratch]`
        code.extend_from_slice(&[0xA1]);
        code.extend_from_slice(&addr32(ctx.scratch_addr).to_le_bytes());
        self.emit_dynamic_jump(code, fixups);

        Ok(())
    }

    fn vmcall(
        &self,
        instruction: &Instruction,
        code: &mut Vec<u8>,
        fixups: &mut Vec<Fixup>,
        ctx: &GenContext,
    ) -> Result<(), VMError> {
        // Any of the three operands may be a register or an immediate.
        let (service, address, size) = instruction.expect3()?;
        let trampoline = addr32(jit_vmcall as *const () as usize as u64);

        // `sub esp, 32` — room for the seven argument slots:
        // `wvm`, `service` (2), `address` (2), `size` (2), `out`.
        // 32 is a multiple of 16, so the stack stays aligned for the call.
        code.extend_from_slice(&[0x83, 0xEC, 0x20]);

        // `mov dword [esp], <wvm>`
        code.extend_from_slice(&[0xC7, 0x44, 0x24, 0x00]);
        code.extend_from_slice(&addr32(ctx.wvm).to_le_bytes());

        let mut operand = Vec::new();
        self.load_eax_edx(&mut operand, service.kind);
        code.extend(operand);
        // `mov [esp + 4], eax`
        code.extend_from_slice(&[0x89, 0x44, 0x24, 0x04]);
        // `mov [esp + 8], edx`
        code.extend_from_slice(&[0x89, 0x54, 0x24, 0x08]);

        let mut operand = Vec::new();
        self.load_eax_edx(&mut operand, address.kind);
        code.extend(operand);
        // `mov [esp + 12], eax`
        code.extend_from_slice(&[0x89, 0x44, 0x24, 0x0C]);
        // `mov [esp + 16], edx`
        code.extend_from_slice(&[0x89, 0x54, 0x24, 0x10]);

        let mut operand = Vec::new();
        self.load_eax_edx(&mut operand, size.kind);
        code.extend(operand);
        // `mov [esp + 20], eax`
        code.extend_from_slice(&[0x89, 0x44, 0x24, 0x14]);
        // `mov [esp + 24], edx`
        code.extend_from_slice(&[0x89, 0x54, 0x24, 0x18]);

        // The out-block pointer at `[esp + 28]`.
        // `mov eax, <out>`
        code.extend_from_slice(&[0xB8]);
        code.extend_from_slice(&addr32(ctx.out).to_le_bytes());
        // `mov [esp + 28], eax`
        code.extend_from_slice(&[0x89, 0x44, 0x24, 0x1C]);

        // `mov eax, <trampoline>`
        code.extend_from_slice(&[0xB8]);
        code.extend_from_slice(&trampoline.to_le_bytes());

        // `call eax`
        code.extend_from_slice(&[0xFF, 0xD0]);

        // `add esp, 32`
        code.extend_from_slice(&[0x83, 0xC4, 0x20]);

        // The `u64` outcome returns in `EDX:EAX`: nonzero means “stop”.
        // `or eax, edx`
        code.extend_from_slice(&[0x09, 0xD0]);

        // `jnz <epilogue>`
        fixups.push(Fixup::RelToEpilogue {
            pos: code.len() + 2,
        });
        code.extend_from_slice(&[0x0F, 0x85, 0, 0, 0, 0]);

        Ok(())
    }

    /// Emits a `Jcc rel32` with a zero placeholder and returns the position
    /// of the displacement field, for branches whose target becomes known
    /// a few bytes later (patched immediately with [`patch_rel32`]).
    fn emit_jcc(code: &mut Vec<u8>, jcc: [u8; 2]) -> usize {
        let pos = code.len() + 2;
        code.extend_from_slice(&[jcc[0], jcc[1], 0, 0, 0, 0]);
        pos
    }

    /// Emits a `jmp rel32` with a zero placeholder; see [`emit_jcc`](Self::emit_jcc).
    fn emit_jmp(code: &mut Vec<u8>) -> usize {
        let pos = code.len() + 1;
        code.extend_from_slice(&[0xE9, 0, 0, 0, 0]);
        pos
    }

    /// Emits an indirect jump to the instruction whose index is in EAX.
    ///
    /// The index is bounds-checked against the address table; an
    /// out-of-range index exits silently through the epilogue, mirroring
    /// the interpreter (`next >= instruction_count` ends execution).
    fn emit_dynamic_jump(&self, code: &mut Vec<u8>, fixups: &mut Vec<Fixup>) {
        // `mov edx, <table base>` — patched once the layout is known.
        fixups.push(Fixup::TableBase {
            pos: code.len() + 1,
        });
        code.extend_from_slice(&[0xBA, 0, 0, 0, 0]);

        // `mov ecx, <table length>` — patched once the layout is known.
        fixups.push(Fixup::TableLen {
            pos: code.len() + 1,
        });
        code.extend_from_slice(&[0xB9, 0, 0, 0, 0]);

        // `cmp eax, ecx`
        code.extend_from_slice(&[0x39, 0xC8]);

        // `jae <epilogue>`
        // `0F 83 <cd>`
        fixups.push(Fixup::RelToEpilogue {
            pos: code.len() + 2,
        });
        code.extend_from_slice(&[0x0F, 0x83, 0, 0, 0, 0]);

        // Load the target's absolute code address and jump to it.
        // `mov eax, [edx + eax * 4]`
        code.extend_from_slice(&[0x8B, 0x04, 0x82]);

        // `jmp eax`
        code.extend_from_slice(&[0xFF, 0xE0]);
    }

    /// Emits a call to the `jit_call_push` trampoline, pushing `ret_idx`
    /// onto the VM call stack. Preserves no registers; the caller spills
    /// live values (see `call_`).
    fn emit_call_push(&self, code: &mut Vec<u8>, ret_idx: usize) {
        let stack = addr32(self.call_stack as u64);
        let trampoline = addr32(jit_call_push as *const () as usize as u64);
        let ret_idx = ret_idx as u32;

        // `sub esp, 16` — room for the two arguments, keeps alignment.
        code.extend_from_slice(&[0x83, 0xEC, 0x10]);

        // `mov dword [esp], <stack>`
        code.extend_from_slice(&[0xC7, 0x44, 0x24, 0x00]);
        code.extend_from_slice(&stack.to_le_bytes());

        // `mov dword [esp + 4], <ret_idx>`
        code.extend_from_slice(&[0xC7, 0x44, 0x24, 0x04]);
        code.extend_from_slice(&ret_idx.to_le_bytes());

        // `mov eax, <trampoline>`
        code.extend_from_slice(&[0xB8]);
        code.extend_from_slice(&trampoline.to_le_bytes());

        // `call eax`
        code.extend_from_slice(&[0xFF, 0xD0]);

        // `add esp, 16`
        code.extend_from_slice(&[0x83, 0xC4, 0x10]);
    }

    // --- Helpers ---

    /// Returns the 32-bit absolute address of a W8 register cell.
    fn register_addr(&self, register: usize) -> u32 {
        addr32(self.register(register) as u64)
    }

    fn mov_eax_immediate(&self, code: &mut Vec<u8>, value: u32) {
        // `mov eax, <immediate>`
        code.extend_from_slice(&[0xB8]);
        code.extend_from_slice(&value.to_le_bytes());
    }

    fn mov_edx_immediate(&self, code: &mut Vec<u8>, value: u32) {
        // `mov edx, <immediate>`
        code.extend_from_slice(&[0xBA]);
        code.extend_from_slice(&value.to_le_bytes());
    }

    fn mov_ecx_immediate(&self, code: &mut Vec<u8>, value: u32) {
        // `mov ecx, <immediate>`
        code.extend_from_slice(&[0xB9]);
        code.extend_from_slice(&value.to_le_bytes());
    }

    fn mov_ebx_immediate(&self, code: &mut Vec<u8>, value: u32) {
        // `mov ebx, <immediate>`
        code.extend_from_slice(&[0xBB]);
        code.extend_from_slice(&value.to_le_bytes());
    }

    fn mov_eax_register(&self, code: &mut Vec<u8>, register: usize) {
        // `mov eax, [absolute address]`
        code.extend_from_slice(&[0xA1]);
        code.extend_from_slice(&self.register_addr(register).to_le_bytes());
    }

    fn mov_edx_register_hi(&self, code: &mut Vec<u8>, register: usize) {
        // `mov edx, [absolute address + 4]`
        let hi = self.register_addr(register).wrapping_add(4);
        code.extend_from_slice(&[0x8B, 0x15]);
        code.extend_from_slice(&hi.to_le_bytes());
    }

    fn mov_ecx_register(&self, code: &mut Vec<u8>, register: usize) {
        // `mov ecx, [absolute address]`
        code.extend_from_slice(&[0x8B, 0x0D]);
        code.extend_from_slice(&self.register_addr(register).to_le_bytes());
    }

    // NOTE: there is deliberately no `mov_ebx_register` reading the cell
    // start. EBX only ever carries the *high* half of a W8 value, so the
    // only correct accessor is `mov_ebx_register_hi`; offering both invites
    // storing `lo:lo`.

    /// Loads an operand value into `EAX` (low 32 bits) and `EDX` (high 32
    /// bits). Clobbers only `EAX` and `EDX`.
    fn load_eax_edx(&self, code: &mut Vec<u8>, operand: OperandKind) {
        match operand {
            OperandKind::Register(register) => {
                self.mov_eax_register(code, register.0 as usize);
                // The high half lives 4 bytes past the cell start.
                self.mov_edx_register_hi(code, register.0 as usize);
            }
            OperandKind::Immediate(value) => {
                self.mov_eax_immediate(code, value as u32);
                self.mov_edx_immediate(code, (value >> 32) as u32);
            }
        }
    }

    /// Loads an operand value into `ECX` (low 32 bits) and `EBX` (high 32
    /// bits). Clobbers only `ECX` and `EBX`.
    fn load_ecx_ebx(&self, code: &mut Vec<u8>, operand: OperandKind) {
        match operand {
            OperandKind::Register(register) => {
                self.mov_ecx_register(code, register.0 as usize);
                self.mov_ebx_register_hi(code, register.0 as usize);
            }
            OperandKind::Immediate(value) => {
                self.mov_ecx_immediate(code, value as u32);
                self.mov_ebx_immediate(code, (value >> 32) as u32);
            }
        }
    }

    fn mov_ebx_register_hi(&self, code: &mut Vec<u8>, register: usize) {
        // `mov ebx, [absolute address + 4]`
        let hi = self.register_addr(register).wrapping_add(4);
        code.extend_from_slice(&[0x8B, 0x1D]);
        code.extend_from_slice(&hi.to_le_bytes());
    }

    /// Stores the 64-bit value in `EAX` (low) and `EDX` (high) into the
    /// destination W8 register.
    fn store_eax_edx(&self, code: &mut Vec<u8>, destination: Register) {
        // `mov [absolute address], eax`
        code.extend_from_slice(&[0xA3]);
        code.extend_from_slice(&self.register_addr(destination.0 as usize).to_le_bytes());
        // `mov [absolute address + 4], edx`
        let hi = self.register_addr(destination.0 as usize).wrapping_add(4);
        code.extend_from_slice(&[0x89, 0x15]);
        code.extend_from_slice(&hi.to_le_bytes());
    }

    fn load_integer_operands(&self, code: &mut Vec<u8>, lhs: OperandKind, rhs: OperandKind) {
        // The left-hand side goes to `EDX:EAX`, the right-hand side to
        // `EBX:ECX`; the loads never cross into each other's registers.
        self.load_eax_edx(code, lhs);
        self.load_ecx_ebx(code, rhs);
    }

    fn load_shift_operands(&self, code: &mut Vec<u8>, lhs: OperandKind, rhs: OperandKind) {
        // The shifted value goes to `EDX:EAX`; only the low 32 bits of the
        // count are used (see `lsl`), so the high half is never loaded.
        self.load_eax_edx(code, lhs);
        match rhs {
            OperandKind::Register(register) => self.mov_ecx_register(code, register.0 as usize),
            OperandKind::Immediate(value) => self.mov_ecx_immediate(code, value as u32),
        }
    }

    fn binary_integer(
        &self,
        instruction: &Instruction,
        operation: fn(&mut Vec<u8>),
    ) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_integer_operands(&mut code, lhs.kind, rhs.kind);
        operation(&mut code);
        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    /// Lowers `IEQ` and `INE`: the result is 1 iff the equality of the two
    /// 64-bit values matches `is_eq`.
    fn eq_ne(&self, instruction: &Instruction, is_eq: bool) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_integer_operands(&mut code, lhs.kind, rhs.kind);

        // Compare the high halves, then the low halves. Any mismatch
        // decides immediately; only two fully equal halves fall through.
        // `cmp edx, ebx`
        code.extend_from_slice(&[0x39, 0xDA]);
        if is_eq {
            // `jne <false>`
            let false_pos = Self::emit_jcc(&mut code, JNE);
            // `cmp eax, ecx`
            code.extend_from_slice(&[0x39, 0xC8]);
            // `jne <false>`
            let false_pos2 = Self::emit_jcc(&mut code, JNE);
            // `<true>` is the fallthrough: `mov eax, 1`
            code.extend_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
            // `jmp <store>`
            let store_pos = Self::emit_jmp(&mut code);
            // `<false>: xor eax, eax`
            let false_off = code.len();
            patch_rel32(&mut code, false_pos, false_off);
            patch_rel32(&mut code, false_pos2, false_off);
            code.extend_from_slice(&[0x31, 0xC0]);
            // `<store>:`
            let store_off = code.len();
            patch_rel32(&mut code, store_pos, store_off);
        } else {
            // `jne <true>`
            let true_pos = Self::emit_jcc(&mut code, JNE);
            // `cmp eax, ecx`
            code.extend_from_slice(&[0x39, 0xC8]);
            // `jne <true>`
            let true_pos2 = Self::emit_jcc(&mut code, JNE);
            // `<false>` is the fallthrough: `xor eax, eax`
            code.extend_from_slice(&[0x31, 0xC0]);
            // `jmp <store>`
            let store_pos = Self::emit_jmp(&mut code);
            // `<true>: mov eax, 1`
            let true_off = code.len();
            patch_rel32(&mut code, true_pos, true_off);
            patch_rel32(&mut code, true_pos2, true_off);
            code.extend_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
            // `<store>:`
            let store_off = code.len();
            patch_rel32(&mut code, store_pos, store_off);
        }

        // `xor edx, edx`
        code.extend_from_slice(&[0x31, 0xD2]);
        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    /// Lowers an ordered 64-bit comparison: the high halves are compared
    /// first (`hi_below` jumps to true, `hi_above` to false), and equal
    /// high halves fall through to the low-half comparison (`lo` jumps to
    /// true, anything else is false).
    fn ordered(
        &self,
        instruction: &Instruction,
        hi_below: [u8; 2],
        hi_above: [u8; 2],
        lo: [u8; 2],
    ) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_integer_operands(&mut code, lhs.kind, rhs.kind);

        // `cmp edx, ebx`
        code.extend_from_slice(&[0x39, 0xDA]);
        // `<hi_below> <true>`
        let true_pos1 = Self::emit_jcc(&mut code, hi_below);
        // `<hi_above> <false>`
        let false_pos1 = Self::emit_jcc(&mut code, hi_above);
        // `cmp eax, ecx`
        code.extend_from_slice(&[0x39, 0xC8]);
        // `<lo> <true>`
        let true_pos2 = Self::emit_jcc(&mut code, lo);
        // `jmp <false>`
        let false_pos2 = Self::emit_jmp(&mut code);
        // `<true>: mov eax, 1`
        let true_off = code.len();
        patch_rel32(&mut code, true_pos1, true_off);
        patch_rel32(&mut code, true_pos2, true_off);
        code.extend_from_slice(&[0xB8, 0x01, 0x00, 0x00, 0x00]);
        // `jmp <store>`
        let store_pos = Self::emit_jmp(&mut code);
        // `<false>: xor eax, eax`
        let false_off = code.len();
        patch_rel32(&mut code, false_pos1, false_off);
        patch_rel32(&mut code, false_pos2, false_off);
        code.extend_from_slice(&[0x31, 0xC0]);
        // `<store>:`
        let store_off = code.len();
        patch_rel32(&mut code, store_pos, store_off);

        // `xor edx, edx`
        code.extend_from_slice(&[0x31, 0xD2]);
        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }

    fn load_float_operands(&self, code: &mut Vec<u8>, lhs: OperandKind, rhs: OperandKind) {
        let mut halves = Vec::new();
        self.load_integer_operands(&mut halves, lhs, rhs);
        code.extend(halves);

        // `sub esp, 16`
        code.extend_from_slice(&[0x83, 0xEC, 0x10]);
        // `mov [esp], eax`; `mov [esp + 4], edx` (lhs bits)
        code.extend_from_slice(&[0x89, 0x04, 0x24]);
        code.extend_from_slice(&[0x89, 0x54, 0x24, 0x04]);
        // `movsd xmm0, [esp]`
        code.extend_from_slice(&[0xF2, 0x0F, 0x10, 0x04, 0x24]);
        // `mov [esp + 8], ecx`; `mov [esp + 12], ebx` (rhs bits)
        code.extend_from_slice(&[0x89, 0x4C, 0x24, 0x08]);
        code.extend_from_slice(&[0x89, 0x5C, 0x24, 0x0C]);
        // `movsd xmm1, [esp + 8]`
        code.extend_from_slice(&[0xF2, 0x0F, 0x10, 0x4C, 0x24, 0x08]);
    }

    fn store_float_result(&self, code: &mut Vec<u8>, destination: Register) {
        // `movsd [esp], xmm0`
        code.extend_from_slice(&[0xF2, 0x0F, 0x11, 0x04, 0x24]);
        // `mov eax, [esp]`
        code.extend_from_slice(&[0x8B, 0x04, 0x24]);
        // `mov edx, [esp + 4]`
        code.extend_from_slice(&[0x8B, 0x54, 0x24, 0x04]);
        // `add esp, 16`
        code.extend_from_slice(&[0x83, 0xC4, 0x10]);
        self.store_eax_edx(code, destination);
    }

    fn binary_float(
        &self,
        instruction: &Instruction,
        operation: fn(&mut Vec<u8>),
    ) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_float_operands(&mut code, lhs.kind, rhs.kind);
        operation(&mut code);
        self.store_float_result(&mut code, destination);

        Ok(code)
    }

    fn binary_fcompare(
        &self,
        instruction: &Instruction,
        condition: &[u8],
    ) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_float_operands(&mut code, lhs.kind, rhs.kind);

        // `ucomisd mm0, xmm1`
        code.extend_from_slice(&[0x66, 0x0F, 0x2E, 0xC1]);

        // Write 1 into AL if the condition holds, 0 otherwise.
        // The caller passes the `setcc` sequence matching the opcode;
        // conditions affected by unordered results combine `setcc al`
        // with a parity-flag check in CL (ECX is free: both operands
        // already live in XMM registers).
        code.extend_from_slice(condition);

        // `movzx eax, al`
        code.extend_from_slice(&[0x0F, 0xB6, 0xC0]);

        // `add esp, 16`
        code.extend_from_slice(&[0x83, 0xC4, 0x10]);
        // `xor edx, edx`
        code.extend_from_slice(&[0x31, 0xD2]);
        self.store_eax_edx(&mut code, destination);

        Ok(code)
    }
}
