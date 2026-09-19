// w8-core/src/vm/jit/codegen/arch/x64.rs
//
//! *See [crate::vm::jit::codegen] module documentation to understand
//! meaning of the “codegen”.*
//!
//! # x64 codegen
//!
//! Machine code generator for the x64 CPU architecture.
//!
//! ## Overview
//!
//! [`JITCodegen::generate`] turns a slice of [`Instruction`]s into a
//! single flat blob of x64 machine code (a [`GeneratedCode`]).
//!
//! Generation is a **two-pass** process:
//!
//! 1. **Pass 1 — emit.** Every W8 instruction is lowered to bytes in
//!    order, one instruction at a time. Its starting offset is recorded in
//!    `offsets[index]`. Instructions with a branch target that cannot be
//!    resolved yet (because the target's final address depends on code
//!    that has not been emitted — later instructions, the epilogue, the
//!    error stub, the address table) instead emit a placeholder (e.g. a
//!    `rel32` field of zero bytes) and push a [`Fixup`] describing what
//!    needs to be written there once the full layout is known.
//! 2. **Pass 2 — patch.** After the error stub, epilogue, and address
//!    table have all been appended, every recorded [`Fixup`] is resolved
//!    and the placeholder bytes are overwritten in place with the real
//!    address.
//!
//! This two-pass shape exists because x86 relative jumps are computed
//! from “end of the jump instruction” to “start of the target”, and both
//! ends are only fully known after the whole function has been laid out.
//!
//! ## Register model
//!
//! Each W8 register lives at a fixed host-memory cell whose absolute
//! address is given by `self.register(index)`. Every read of a W8
//! register is a `mov` through that absolute address (the x64 `moffs64`
//! addressing form, which only works through `RAX`); every write is the
//! same in reverse. See [`mov_rax_register`](JITCodegen::mov_rax_register),
//! [`mov_rcx_register`](JITCodegen::mov_rcx_register), and
//! [`mov_absolute_rax`](JITCodegen::mov_absolute_rax).
//!
//! ## Control flow: direct vs. dynamic, and the address table
//!
//! A W8 branch target is either an **immediate** instruction index (known
//! at codegen time) or a **register** holding the index at runtime
//! (dynamic — the target is not known until the generated code actually
//! runs).
//!
//! Dynamic targets go through the **address table**: a flat array of
//! absolute code addresses, one per W8 instruction, appended after the
//! epilogue's `ret`. [`emit_dynamic_jump`](JITCodegen::emit_dynamic_jump)
//! bounds-checks the runtime index against the table length and either
//! loads `table[index]` and jumps to it, or — on an out-of-range index —
//! falls through to the epilogue.
//!
//! ## Calls, returns, and the `VMCALL` stack
//!
//! `CALL` and `RET` do not use the x86 call stack. Instead they push or pop
//! instruction indices on a VM-owned `Vec<usize>` (`self.call_stack`),
//! via two `extern "C"` trampolines called from generated code:
//! [`jit_call_push`] and [`jit_call_pop`]. `RET` on an empty stack
//! reports [`VmcallOut::EMPTY_CALL_STACK`] through the shared *out* block
//! and falls into the **error stub**.
//!
//! ## VMCALL and the host boundary
//!
//! `VMCALL` calls [`jit_vmcall`]. Its also refreshes the *membase* cell,
//! because a host callback may replace the VM's memory buffer — `LOAD*`
//! and `STORE*` never bake the base pointer into the generated code;
//! they always re-read it from `ctx.membase_addr`. A nonzero trampoline
//! result means “stop”: generated code tests `RAX` and branches straight
//! to the epilogue.
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

/// A pending patch in the generated machine code, resolved in pass 2 once
/// the full layout (instruction offsets, error stub, epilogue, data cells)
/// is known.
enum Fixup {
    /// A 4-byte `rel32` field at `pos` jumping to the start of instruction
    /// `target`. Out-of-range targets land on the epilogue instead.
    RelToInstr { pos: usize, target: usize },

    /// A 4-byte `rel32` field at `pos` jumping to the epilogue.
    RelToEpilogue { pos: usize },

    /// A 4-byte `rel32` field at `pos` jumping to the `RET`-empty error stub.
    RelToErrorStub { pos: usize },

    /// An 8-byte absolute-address field at `pos` holding the address-table base.
    TableBase { pos: usize },

    /// An 8-byte field at `pos` holding the address-table length.
    TableLen { pos: usize },
}

/// Writes a `rel32` displacement jumping from the end of the 4-byte field at
/// `pos` to `target_off`.
fn patch_rel32(code: &mut [u8], pos: usize, target_off: usize) {
    let displacement = i32::try_from(target_off as i64 - (pos as i64 + 4))
        .expect("x64 jump displacement out of `i32` range");
    code[pos..pos + 4].copy_from_slice(&displacement.to_le_bytes());
}

/// Whether the instruction jumps to a register-held target (an address known
/// only at runtime, needing the address table).
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

/// Pushes a return address onto the VM call stack.
///
/// Called from JIT-generated `CALL` code; `stack` is the call-stack pointer
/// captured by the code generator.
extern "C" fn jit_call_push(stack: *mut Vec<usize>, ret_idx: usize) {
    // SAFETY: generated code passes the live VM call stack embedded at
    // codegen time; `run` does not return until the generated code finishes,
    // so the `Vec` outlives the call.
    unsafe {
        (*stack).push(ret_idx);
    }
}

/// Pops a return address from the VM call stack.
///
/// Writes the popped index to `out` and returns 1, or returns 0 when the
/// stack is empty.
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

/// Handles a `VMCALL` from generated code: an exact port of the
/// interpreter's `vmcall_variant!` (bounds check, dispatcher take/restore,
/// decision), plus a refresh of the memory-base cell (the host may replace
/// the buffer).
///
/// Returns the outcome code (`VmcallOut::*`) and persists it with any details
/// into `out`; generated code branches to the epilogue on nonzero.
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

impl JITCodegen {
    pub fn generate(
        &mut self,
        instructions: &[Instruction],
        ctx: &GenContext,
    ) -> Result<GeneratedCode, VMError> {
        // Pre-scan: dynamic control flow needs the address table after the
        // code; `RET` additionally needs the error stub. Any `RET` or
        // `VMCALL` means execution can report into the out block.
        let need_error_stub = instructions
            .iter()
            .any(|i| matches!(i.opcode, OperationCode::RET));
        let need_table = need_error_stub || instructions.iter().any(has_register_target);
        let has_vmcall = instructions
            .iter()
            .any(|i| matches!(i.opcode, OperationCode::VMCALL));
        let has_status = need_error_stub || has_vmcall;

        // Pass 1: emit machine code, remembering each instruction's offset
        // and collecting patches for pass 2.
        let mut machine_code = Vec::new();
        let mut offsets: Vec<usize> = Vec::with_capacity(instructions.len());
        let mut fixups: Vec<Fixup> = Vec::new();

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
                OperationCode::SDIV => self.sdiv(instruction)?,
                OperationCode::UDIV => self.udiv(instruction)?,
                OperationCode::SREM => self.srem(instruction)?,
                OperationCode::UREM => self.urem(instruction)?,
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

        // A program containing `RET` has the error stub right after the
        // last instruction: falling through the end would land in it and
        // spuriously report `EMPTY_CALL_STACK`. A trailing jump to the
        // epilogue closes that path, mirroring the interpreter (running
        // past the end exits silently). It is skipped when the last
        // instruction never falls through (`JMP`, `CALL`, `RET`).
        //
        // `jmp rel32`
        if need_error_stub
            && instructions.last().is_some_and(|i| {
                !matches!(
                    i.opcode,
                    OperationCode::JMP | OperationCode::CALL | OperationCode::RET
                )
            })
        {
            fixups.push(Fixup::RelToEpilogue {
                pos: machine_code.len() + 1,
            });
            machine_code.extend_from_slice(&[0xE9, 0, 0, 0, 0]);
        }

        // Error stub: `RET` on an empty call stack reports the failure and
        // falls through to the final `ret`. The out block address is known
        // at emission time (`status` is its first field).
        let error_stub = if need_error_stub {
            let stub = machine_code.len();
            // `mov rax, EMPTY_CALL_STACK`
            machine_code.extend_from_slice(&[0x48, 0xC7, 0xC0]);
            machine_code.extend_from_slice(&(VmcallOut::EMPTY_CALL_STACK as u32).to_le_bytes());
            // `mov [<out.status absolute address>], rax`
            machine_code.extend_from_slice(&[0x48, 0xA3]);
            machine_code.extend_from_slice(&ctx.out.to_le_bytes());
            Some(stub)
        } else {
            None
        };

        // Return control to the caller. Out-of-range jumps land
        // here, mirroring the interpreter (`next >= instruction_count` ends
        // execution).
        //
        // `ret`
        // `C3`
        let epilogue = machine_code.len();
        machine_code.push(0xC3);

        // The address table follows the final `ret`, 8-byte aligned
        // (padding is never executed). Status/scratch cells are NOT placed
        // here: the mapping becomes read-execute before execution, so
        // everything the generated code writes lives in `ctx` instead (W^X).
        let table_base = if need_table {
            while machine_code.len() % 8 != 0 {
                machine_code.push(0x90); //< `nop`
            }
            let table_base = ctx.code_base + machine_code.len() as u64;
            for &offset in &offsets {
                machine_code.extend_from_slice(&(ctx.code_base + offset as u64).to_le_bytes());
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
                Fixup::TableBase { pos } => machine_code[pos..pos + 8].copy_from_slice(
                    &table_base
                        .expect("table fixup without a table")
                        .to_le_bytes(),
                ),
                Fixup::TableLen { pos } => machine_code[pos..pos + 8]
                    .copy_from_slice(&(offsets.len() as u64).to_le_bytes()),
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
            (Some(op1), Some(op2), None) => match (op1.kind, op2.kind) {
                // `MOVE <register>, <immediate>`
                //
                // ->
                //
                // ```x86asm
                // mov rax, <immediate>
                // mov [<register absolute address>], rax
                // ```
                //
                // We use RAX as an intermediate register because the x64 ISA does not
                // provide a `mov r/m64, imm64` encoding. The `moffs64` form of `mov`
                // supports an absolute 64-bit memory address, but only with RAX.
                // Therefore, the immediate value is first loaded into RAX and then
                // stored at the absolute address of the W8 register.
                (OperandKind::Register(r), OperandKind::Immediate(v)) => {
                    self.mov_rax_immediate(&mut code, v);
                    self.mov_absolute_rax(&mut code, r.0 as usize);

                    Ok(code)
                }

                // `MOVE <register>, <register>`
                //
                // ->
                //
                // ```x86asm
                // mov rax, [<source register absolute address>]
                // mov [<destination register absolute address>], rax
                // ```
                //
                // The source register is loaded into RAX first, because the x64
                // `mov` instruction does not provide a direct memory-to-memory form.
                // RAX is then used as an intermediate register to store the value
                // into the destination register.
                (OperandKind::Register(r), OperandKind::Register(v)) => {
                    self.mov_rax_register(&mut code, v.0 as usize);
                    self.mov_absolute_rax(&mut code, r.0 as usize);

                    Ok(code)
                }

                // The first operand of `MOVE` must always be a register.
                (got, _) => Err(VMError::new(VMErrorKind::IncorrectTypeOfOperand {
                    expected: OperandKind::Register(Register(0)),
                    got,
                })),
            },

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
            // `add rax, rcx`
            code.extend_from_slice(&[0x48, 0x01, 0xC8]);
        })
    }

    fn isub(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `sub rax, rcx`
            code.extend_from_slice(&[0x48, 0x29, 0xC8]);
        })
    }

    fn imul(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `imul rax, rcx`
            code.extend_from_slice(&[0x48, 0x0F, 0xAF, 0xC1]);
        })
    }

    fn sdiv(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // Sign-extend RAX into RDX:RAX before signed division.
            // `cqo`
            code.extend_from_slice(&[0x48, 0x99]);

            // Divide the signed 128-bit value in RDX:RAX by RCX; quotient stays in RAX.
            // `idiv rcx`
            code.extend_from_slice(&[0x48, 0xF7, 0xF9]);
        })
    }

    fn udiv(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // Clear the high half of the unsigned dividend in RDX:RAX.
            // `xor edx, edx`
            code.extend_from_slice(&[0x31, 0xD2]);

            // Divide RDX:RAX by RCX; quotient stays in RAX.
            // `div rcx`
            code.extend_from_slice(&[0x48, 0xF7, 0xF1]);
        })
    }

    fn srem(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `cqo`
            code.extend_from_slice(&[0x48, 0x99]);

            // Signed division leaves the remainder in RDX.
            // `idiv rcx`
            code.extend_from_slice(&[0x48, 0xF7, 0xF9]);

            // Move the remainder to RAX before storing it in the destination register.
            // `mov rax, rdx`
            code.extend_from_slice(&[0x48, 0x89, 0xD0]);
        })
    }

    fn urem(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `xor edx, edx`
            code.extend_from_slice(&[0x31, 0xD2]);

            // Unsigned division leaves the remainder in RDX.
            // `div rcx`
            code.extend_from_slice(&[0x48, 0xF7, 0xF1]);

            // Move the remainder to RAX before storing it in the destination register.
            // `mov rax, rdx`
            code.extend_from_slice(&[0x48, 0x89, 0xD0]);
        })
    }

    fn ineg(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, source) = instruction.expect2()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        match source.kind {
            // `mov rax, [absolute address]`
            OperandKind::Register(register) => {
                self.mov_rax_register(&mut code, register.0 as usize)
            }

            // `mov rax, <immediate>`
            OperandKind::Immediate(value) => self.mov_rax_immediate(&mut code, value),
        }

        // Negation is wrapping for two's-complement integer values.
        // `neg rax`
        code.extend_from_slice(&[0x48, 0xF7, 0xD8]);
        self.store_integer_result(&mut code, destination);

        Ok(code)
    }

    fn and_(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `and rax, rcx`
            code.extend_from_slice(&[0x48, 0x21, 0xC8]);
        })
    }

    fn or_(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `or rax, rcx`
            code.extend_from_slice(&[0x48, 0x09, 0xC8]);
        })
    }

    fn xor_(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // `xor rax, rcx`
            code.extend_from_slice(&[0x48, 0x31, 0xC8]);
        })
    }

    fn not_(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, source) = instruction.expect2()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        match source.kind {
            // `mov rax, [absolute address]`
            OperandKind::Register(register) => {
                self.mov_rax_register(&mut code, register.0 as usize)
            }

            // `mov rax, <immediate>`
            OperandKind::Immediate(value) => self.mov_rax_immediate(&mut code, value),
        }

        // Bitwise inversion.
        // `not rax`
        code.extend_from_slice(&[0x48, 0xF7, 0xD0]);
        self.store_integer_result(&mut code, destination);

        Ok(code)
    }

    fn lsl(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // Logical shift left by the count in CL.
            //
            // The 64-bit shift count is masked to 6 bits by the CPU,
            // matching `wrapping_shl` in the interpreter.
            //
            // `shl rax, cl`
            code.extend_from_slice(&[0x48, 0xD3, 0xE0]);
        })
    }

    fn lsr(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // Logical shift right by the count in CL.
            //
            // The 64-bit shift count is masked to 6 bits by the CPU,
            // matching `wrapping_shr` in the interpreter.
            //
            // `shr rax, cl`
            code.extend_from_slice(&[0x48, 0xD3, 0xE8]);
        })
    }

    fn sar(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        self.binary_integer(instruction, |code| {
            // Arithmetic shift right by the count in CL (sign bit is preserved).
            //
            // The 64-bit shift count is masked to 6 bits by the CPU,
            // matching `wrapping_shr` on `i64` in the interpreter.
            //
            // `sar rax, cl`
            code.extend_from_slice(&[0x48, 0xD3, 0xF8]);
        })
    }

    fn ieq(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // `sete al`
        // `0F 94 C0`
        self.binary_compare(instruction, &[0x0F, 0x94, 0xC0])
    }

    fn ine(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // `setne al`
        // `0F 95 C0`
        self.binary_compare(instruction, &[0x0F, 0x95, 0xC0])
    }

    fn slt(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Signed less-than.
        // `setl al`
        // `0F 9C C0`
        self.binary_compare(instruction, &[0x0F, 0x9C, 0xC0])
    }

    fn sle(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Signed less-than or equal.
        // `setle al`
        // `0F 9E C0`
        self.binary_compare(instruction, &[0x0F, 0x9E, 0xC0])
    }

    fn sgt(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Signed greater-than.
        // `setg al`
        // `0F 9F C0`
        self.binary_compare(instruction, &[0x0F, 0x9F, 0xC0])
    }

    fn sge(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Signed greater-than or equal.
        // `setge al`
        // `0F 9D C0`
        self.binary_compare(instruction, &[0x0F, 0x9D, 0xC0])
    }

    fn ult(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Unsigned less-than (carry flag set).
        // `setb al`
        // `0F 92 C0`
        self.binary_compare(instruction, &[0x0F, 0x92, 0xC0])
    }

    fn ule(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Unsigned less-than or equal.
        // `setbe al`
        // `0F 96 C0`
        self.binary_compare(instruction, &[0x0F, 0x96, 0xC0])
    }

    fn ugt(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Unsigned greater-than.
        // `seta al`
        // `0F 97 C0`
        self.binary_compare(instruction, &[0x0F, 0x97, 0xC0])
    }

    fn uge(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        // Unsigned greater-than or equal.
        // `setae al`
        // `0F 93 C0`
        self.binary_compare(instruction, &[0x0F, 0x93, 0xC0])
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

        self.load_float_operands(&mut code, lhs.kind, rhs.kind);
        // Save both operands on the stack for the x87 remainder instruction.
        // `sub rsp, 16`
        code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x10]);
        self.movq_rax_xmm0(&mut code);
        // `mov [rsp], rax`
        code.extend_from_slice(&[0x48, 0x89, 0x04, 0x24]);
        self.movq_rcx_xmm1(&mut code);
        // `mov [rsp + 8], rcx`
        code.extend_from_slice(&[0x48, 0x89, 0x4C, 0x24, 0x08]);

        // Load rhs first and lhs second so FPREM computes ST0 % ST1 as lhs % rhs.
        // `fld qword [rsp + 8]`
        code.extend_from_slice(&[0xDD, 0x44, 0x24, 0x08]);
        // `fld qword [rsp]`
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
        // `fstp qword [rsp]`
        code.extend_from_slice(&[0xDD, 0x1C, 0x24]);
        // Discard the divisor left in ST0.
        // `fstp st0`
        code.extend_from_slice(&[0xDD, 0xD8]);
        // `mov rax, [rsp]`
        code.extend_from_slice(&[0x48, 0x8B, 0x04, 0x24]);
        // `add rsp, 16`
        code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x10]);
        self.mov_absolute_rax(&mut code, destination.0 as usize);

        Ok(code)
    }

    fn fneg(&self, instruction: &Instruction) -> Result<Vec<u8>, VMError> {
        let (destination, source) = instruction.expect2()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        match source.kind {
            OperandKind::Register(register) => {
                self.mov_rax_register(&mut code, register.0 as usize)
            }
            OperandKind::Immediate(value) => self.mov_rax_immediate(&mut code, value),
        }
        self.movq_xmm0_rax(&mut code);

        // Load the sign-bit mask into RCX and transfer it to XMM1.
        // `mov rcx, 8000000000000000h`
        // `48 B9 <imm64>`
        self.mov_rcx_immediate(&mut code, 0x8000_0000_0000_0000);
        self.movq_xmm1_rcx(&mut code);
        // Flip only the sign bit to negate the f64 value.
        // `xorpd xmm0, xmm1`
        code.extend_from_slice(&[0x66, 0x0F, 0x57, 0xC1]);
        self.store_float_result(&mut code, destination);

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
        // Ordered less-than or equal: CF/ZF cover unordered too, so exclude
        // it via the parity flag.
        //
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

        // Load the VM memory address (offset) into RAX.
        //
        // `LOAD* <dst>, <src>` reads from `src` as an address:
        // - register -> address is stored in the W8 register;
        // - immediate -> address is the immediate itself.
        match source.kind {
            // `mov rax, [absolute address]`
            // `48 A1 <moffs64>`
            OperandKind::Register(register) => {
                self.mov_rax_register(&mut code, register.0 as usize)
            }

            // `mov rax, <immediate>`
            // `48 B8 <imm64>`
            OperandKind::Immediate(value) => self.mov_rax_immediate(&mut code, value),
        }

        // Load the current base pointer of the VM memory into RDX.
        //
        // The base lives in a cell (refreshed by the `VMCALL` trampoline
        // after every host call), never baked in: a host may replace the
        // buffer mid-run.
        //
        // `mov rdx, <membase cell>`
        code.extend_from_slice(&[0x48, 0xBA]);
        code.extend_from_slice(&ctx.membase_addr.to_le_bytes());
        // `mov rdx, [rdx]`
        code.extend_from_slice(&[0x48, 0x8B, 0x12]);

        // Add the requested address to the base pointer.
        //
        // `add rdx, rax`
        //
        // After this RDX points at the first byte to read.
        code.extend_from_slice(&[0x48, 0x01, 0xC2]);

        // Load from `[rdx]` with zero-extension into RAX.
        //
        // Writing EAX clears the upper 32 bits of RAX, so 8/16/32-bit loads
        // are already zero-extended to 64 bits, matching the interpreter
        // (`u64::from(value)` in `load_variant!`).
        match instruction.opcode {
            // `movzx eax, byte [rdx]`
            OperationCode::LOAD8 => code.extend_from_slice(&[0x0F, 0xB6, 0x02]),

            // `movzx eax, word [rdx]`
            OperationCode::LOAD16 => code.extend_from_slice(&[0x0F, 0xB7, 0x02]),

            // `mov eax, dword [rdx]`
            OperationCode::LOAD32 => code.extend_from_slice(&[0x8B, 0x02]),

            // `mov rax, qword [rdx]`
            OperationCode::LOAD64 => code.extend_from_slice(&[0x48, 0x8B, 0x02]),

            _ => unimplemented!(),
        }

        // Store the zero-extended value into the destination register.
        //
        // `mov [absolute address], rax`
        self.mov_absolute_rax(&mut code, destination.0 as usize);

        Ok(code)
    }

    fn store_(&self, instruction: &Instruction, ctx: &GenContext) -> Result<Vec<u8>, VMError> {
        let (address, value) = instruction.expect2()?;
        let mut code = Vec::new();

        // Load the VM memory address (offset) into RAX.
        //
        // `STORE* <dst>, <src>` writes to `dst` as an address:
        // - register -> address is stored in the W8 register;
        // - immediate -> address is the immediate itself.
        match address.kind {
            // `mov rax, [absolute address]`
            OperandKind::Register(register) => {
                self.mov_rax_register(&mut code, register.0 as usize)
            }

            // `mov rax, <immediate>`
            OperandKind::Immediate(value) => self.mov_rax_immediate(&mut code, value),
        }

        // Load the value to store into RCX.
        //
        // Only the low 8/16/32/64 bits are written, matching the
        // interpreter truncation (`as u8/u16/u32/u64` in `store_variant!`).
        // Must come after the address load: `mov_rcx_register` clobbers
        // RDX internally, which does not hold the memory base yet.
        match value.kind {
            // ```x86asm
            // mov rdx, <absolute address>
            // mov rcx, [rdx]
            // ```
            OperandKind::Register(register) => {
                self.mov_rcx_register(&mut code, register.0 as usize)
            }

            // `mov rcx, <immediate>`
            OperandKind::Immediate(value) => self.mov_rcx_immediate(&mut code, value),
        }

        // Load the current base pointer of the VM memory into RDX.
        //
        // The base lives in a cell (refreshed by the `VMCALL` trampoline
        // after every host call), never baked in: a host may replace the
        // buffer mid-run.
        //
        // `mov rdx, <membase cell>`
        code.extend_from_slice(&[0x48, 0xBA]);
        code.extend_from_slice(&ctx.membase_addr.to_le_bytes());
        // `mov rdx, [rdx]`
        code.extend_from_slice(&[0x48, 0x8B, 0x12]);

        // Add the requested address to the base pointer.
        //
        // `add rdx, rax`
        //
        // After this RDX points at the first byte to write.
        code.extend_from_slice(&[0x48, 0x01, 0xC2]);

        // Store the low part of RCX into `[rdx]`.
        match instruction.opcode {
            // `mov [rdx], cl`
            OperationCode::STORE8 => code.extend_from_slice(&[0x88, 0x0A]),

            // `mov [rdx], cx`
            OperationCode::STORE16 => code.extend_from_slice(&[0x66, 0x89, 0x0A]),

            // `mov [rdx], ecx`
            OperationCode::STORE32 => code.extend_from_slice(&[0x89, 0x0A]),

            // `mov [rdx], rcx`
            OperationCode::STORE64 => code.extend_from_slice(&[0x48, 0x89, 0x0A]),

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
                self.mov_rax_register(code, register.0 as usize);
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
                match condition.kind {
                    OperandKind::Register(register) => {
                        self.mov_rax_register(code, register.0 as usize)
                    }
                    OperandKind::Immediate(v) => self.mov_rax_immediate(code, v),
                }

                // `test rax, rax`
                code.extend_from_slice(&[0x48, 0x85, 0xC0]);

                // `jz rel32`
                fixups.push(Fixup::RelToInstr {
                    pos: code.len() + 2,
                    target: value as usize,
                });
                code.extend_from_slice(&[0x0F, 0x84, 0, 0, 0, 0]);
            }

            OperandKind::Register(register) => {
                // Condition goes to RCX first (`mov_rcx_register` clobbers
                // RDX, which holds nothing yet), the target index to RAX.
                match condition.kind {
                    OperandKind::Register(condition) => {
                        self.mov_rcx_register(code, condition.0 as usize)
                    }
                    OperandKind::Immediate(v) => self.mov_rcx_immediate(code, v),
                }
                self.mov_rax_register(code, register.0 as usize);

                // Skip the dynamic jump when the condition is nonzero.
                // `test rcx, rcx`
                code.extend_from_slice(&[0x48, 0x85, 0xC9]);

                // `jnz <next>`
                let skip_pos = code.len() + 2;
                code.extend_from_slice(&[0x0F, 0x85, 0, 0, 0, 0]);
                self.emit_dynamic_jump(code, fixups);
                let next = code.len();
                patch_rel32(code, skip_pos, next);
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
                match condition.kind {
                    OperandKind::Register(register) => {
                        self.mov_rax_register(code, register.0 as usize)
                    }
                    OperandKind::Immediate(v) => self.mov_rax_immediate(code, v),
                }

                // `test rax, rax`
                code.extend_from_slice(&[0x48, 0x85, 0xC0]);

                // `jnz rel32`
                fixups.push(Fixup::RelToInstr {
                    pos: code.len() + 2,
                    target: value as usize,
                });
                code.extend_from_slice(&[0x0F, 0x85, 0, 0, 0, 0]);
            }

            OperandKind::Register(register) => {
                // Same layout as `jz_` with the skip condition inverted.
                match condition.kind {
                    OperandKind::Register(condition) => {
                        self.mov_rcx_register(code, condition.0 as usize)
                    }
                    OperandKind::Immediate(v) => self.mov_rcx_immediate(code, v),
                }
                self.mov_rax_register(code, register.0 as usize);

                // `test rcx, rcx`
                code.extend_from_slice(&[0x48, 0x85, 0xC9]);

                // Skip the dynamic jump when the condition is zero.
                // `jz <next>`
                let skip_pos = code.len() + 2;
                code.extend_from_slice(&[0x0F, 0x84, 0, 0, 0, 0]);
                self.emit_dynamic_jump(code, fixups);
                let next = code.len();
                patch_rel32(code, skip_pos, next);
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
                self.mov_rax_register(code, register.0 as usize);

                // Spill the target index across the trampoline call
                // (`call` leaves RAX unspecified).
                // `push rax`
                code.push(0x50);
                self.emit_call_push(code, ret_idx);

                // `pop rax`
                code.push(0x58);
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

        let stack = self.call_stack as u64;
        let trampoline = jit_call_pop as *const () as usize as u64;

        #[cfg(unix)]
        {
            // `sub rsp, 8` — align the stack for the call.
            code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x08]);
            // `mov rdi, <stack>`
            code.extend_from_slice(&[0x48, 0xBF]);
            code.extend_from_slice(&stack.to_le_bytes());

            // `mov rsi, <scratch>` — a `JIT`-owned cell, address known upfront.
            code.extend_from_slice(&[0x48, 0xBE]);
            code.extend_from_slice(&ctx.scratch_addr.to_le_bytes());

            // `mov rax, <trampoline>`
            code.extend_from_slice(&[0x48, 0xB8]);
            code.extend_from_slice(&trampoline.to_le_bytes());

            // `call rax`
            code.extend_from_slice(&[0xFF, 0xD0]);

            // `add rsp, 8`
            code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08]);
        }

        #[cfg(windows)]
        {
            // `sub rsp, 40` — 32 bytes of shadow space plus alignment.
            code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);

            // `mov rcx, <stack>`
            code.extend_from_slice(&[0x48, 0xB9]);
            code.extend_from_slice(&stack.to_le_bytes());

            // `mov rdx, <scratch>` — a `JIT`-owned cell, address known upfront.
            code.extend_from_slice(&[0x48, 0xBA]);
            code.extend_from_slice(&ctx.scratch_addr.to_le_bytes());

            // `mov rax, <trampoline>`
            code.extend_from_slice(&[0x48, 0xB8]);
            code.extend_from_slice(&trampoline.to_le_bytes());

            // `call rax`
            code.extend_from_slice(&[0xFF, 0xD0]);

            // `add rsp, 40`
            code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x28]);
        }

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

        // `mov rax, [scratch]`
        code.extend_from_slice(&[0x48, 0xA1]);
        code.extend_from_slice(&ctx.scratch_addr.to_le_bytes());
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
        let trampoline = jit_vmcall as *const () as usize as u64;

        #[cfg(unix)]
        {
            // `sub rsp, 8` — align the stack for the call.
            code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x08]);
            // `mov rdi, <w8>`
            code.extend_from_slice(&[0x48, 0xBF]);
            code.extend_from_slice(&ctx.wvm.to_le_bytes());

            self.mov_rax_operand(code, service.kind);

            // `mov rsi, rax`
            code.extend_from_slice(&[0x48, 0x89, 0xC6]);

            self.mov_rax_operand(code, address.kind);

            // `mov rdx, rax`
            code.extend_from_slice(&[0x48, 0x89, 0xC2]);

            self.mov_rax_operand(code, size.kind);

            // `mov rcx, rax`
            code.extend_from_slice(&[0x48, 0x89, 0xC1]);

            // `mov r8, <out>`
            code.extend_from_slice(&[0x49, 0xB8]);
            code.extend_from_slice(&ctx.out.to_le_bytes());

            // `mov rax, <trampoline>`
            code.extend_from_slice(&[0x48, 0xB8]);
            code.extend_from_slice(&trampoline.to_le_bytes());

            // `call rax`
            code.extend_from_slice(&[0xFF, 0xD0]);

            // `add rsp, 8`
            code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08]);
        }

        #[cfg(windows)]
        {
            // `sub rsp, 40` — 32 bytes of shadow space plus alignment; the
            // fifth argument rides at `[rsp + 32]`.
            code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);

            // `mov rcx, <w8>`
            code.extend_from_slice(&[0x48, 0xB9]);
            code.extend_from_slice(&ctx.wvm.to_le_bytes());

            self.mov_rax_operand(code, service.kind);

            // `mov rdx, rax`
            code.extend_from_slice(&[0x48, 0x89, 0xC2]);

            self.mov_rax_operand(code, address.kind);

            // `mov r8, rax`
            code.extend_from_slice(&[0x49, 0x89, 0xC0]);

            self.mov_rax_operand(code, size.kind);

            // `mov r9, rax`
            code.extend_from_slice(&[0x49, 0x89, 0xC1]);

            // ```
            // mov rax, <out>
            // mov [rsp + 32], rax
            // ```
            code.extend_from_slice(&[0x48, 0xB8]);
            code.extend_from_slice(&ctx.out.to_le_bytes());
            code.extend_from_slice(&[0x48, 0x89, 0x44, 0x24, 0x20]);

            // `mov rax, <trampoline>`
            code.extend_from_slice(&[0x48, 0xB8]);
            code.extend_from_slice(&trampoline.to_le_bytes());

            // `call rax`
            code.extend_from_slice(&[0xFF, 0xD0]);

            // `add rsp, 40`
            code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x28]);
        }

        // `test rax, rax`
        code.extend_from_slice(&[0x48, 0x85, 0xC0]);

        // `jnz <epilogue>`
        fixups.push(Fixup::RelToEpilogue {
            pos: code.len() + 2,
        });
        code.extend_from_slice(&[0x0F, 0x85, 0, 0, 0, 0]);

        Ok(())
    }

    /// Loads an operand value (register contents or immediate) into RAX.
    fn mov_rax_operand(&self, code: &mut Vec<u8>, operand: OperandKind) {
        match operand {
            // `mov rax, [absolute address]`
            OperandKind::Register(register) => self.mov_rax_register(code, register.0 as usize),

            // `mov rax, <immediate>`
            OperandKind::Immediate(value) => self.mov_rax_immediate(code, value),
        }
    }

    /// Emits an indirect jump to the instruction whose index is in RAX.
    ///
    /// The index is bounds-checked against the address table; an
    /// out-of-range index exits silently through the epilogue, mirroring
    /// the interpreter (`next >= instruction_count` ends execution).
    fn emit_dynamic_jump(&self, code: &mut Vec<u8>, fixups: &mut Vec<Fixup>) {
        // `mov rdx, <table base>` — patched once the layout is known.
        fixups.push(Fixup::TableBase {
            pos: code.len() + 2,
        });
        code.extend_from_slice(&[0x48, 0xBA, 0, 0, 0, 0, 0, 0, 0, 0]);

        // `mov rcx, <table length>` — patched once the layout is known.
        fixups.push(Fixup::TableLen {
            pos: code.len() + 2,
        });
        code.extend_from_slice(&[0x48, 0xB9, 0, 0, 0, 0, 0, 0, 0, 0]);

        // `cmp rax, rcx`
        code.extend_from_slice(&[0x48, 0x39, 0xC8]);

        // `jae <epilogue>`
        // `0F 83 <cd>`
        fixups.push(Fixup::RelToEpilogue {
            pos: code.len() + 2,
        });
        code.extend_from_slice(&[0x0F, 0x83, 0, 0, 0, 0]);

        // Load the target's absolute code address and jump to it.
        // `mov rax, [rdx + rax * 8]`
        code.extend_from_slice(&[0x48, 0x8B, 0x04, 0xC2]);

        // `jmp rax`
        code.extend_from_slice(&[0xFF, 0xE0]);
    }

    /// Emits a call to the `jit_call_push` trampoline, pushing `ret_idx`
    /// onto the VM call stack. Preserves no registers; the caller spills
    /// live values (see `call_`).
    fn emit_call_push(&self, code: &mut Vec<u8>, ret_idx: usize) {
        let stack = self.call_stack as u64;
        let trampoline = jit_call_push as *const () as usize as u64;
        let ret_idx = ret_idx as u64;

        #[cfg(unix)]
        {
            // `sub rsp, 8` — align the stack for the call.
            code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x08]);

            // `mov rdi, <stack>`
            code.extend_from_slice(&[0x48, 0xBF]);
            code.extend_from_slice(&stack.to_le_bytes());

            // `mov rsi, <ret_idx>`
            code.extend_from_slice(&[0x48, 0xBE]);
            code.extend_from_slice(&ret_idx.to_le_bytes());

            // `mov rax, <trampoline>`
            code.extend_from_slice(&[0x48, 0xB8]);
            code.extend_from_slice(&trampoline.to_le_bytes());

            // `call rax`
            code.extend_from_slice(&[0xFF, 0xD0]);

            // `add rsp, 8`
            code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x08]);
        }

        #[cfg(windows)]
        {
            // `sub rsp, 40` — 32 bytes of shadow space plus alignment.
            code.extend_from_slice(&[0x48, 0x83, 0xEC, 0x28]);

            // `mov rcx, <stack>`
            code.extend_from_slice(&[0x48, 0xB9]);
            code.extend_from_slice(&stack.to_le_bytes());

            // `mov rdx, <ret_idx>`
            code.extend_from_slice(&[0x48, 0xBA]);
            code.extend_from_slice(&ret_idx.to_le_bytes());

            // `mov rax, <trampoline>`
            code.extend_from_slice(&[0x48, 0xB8]);
            code.extend_from_slice(&trampoline.to_le_bytes());

            // `call rax`
            code.extend_from_slice(&[0xFF, 0xD0]);

            // `add rsp, 40`
            code.extend_from_slice(&[0x48, 0x83, 0xC4, 0x28]);
        }
    }

    // --- Helpers ---

    fn mov_rax_immediate(&self, code: &mut Vec<u8>, value: u64) {
        // `mov rax, <immediate>`
        code.extend_from_slice(&[0x48, 0xB8]);
        code.extend_from_slice(&value.to_le_bytes());
    }

    fn mov_rax_register(&self, code: &mut Vec<u8>, register: usize) {
        // `mov rax, [absolute address]`
        code.extend_from_slice(&[0x48, 0xA1]);
        code.extend_from_slice(&(self.register(register) as u64).to_le_bytes());
    }

    fn mov_rcx_immediate(&self, code: &mut Vec<u8>, value: u64) {
        // `mov rcx, <immediate>`
        code.extend_from_slice(&[0x48, 0xB9]);
        code.extend_from_slice(&value.to_le_bytes());
    }

    fn mov_rcx_register(&self, code: &mut Vec<u8>, register: usize) {
        // Load the register value into RCX through RDX, because RAX contains the other operand.

        // `mov rdx, <absolute address>`
        code.extend_from_slice(&[0x48, 0xBA]);
        code.extend_from_slice(&(self.register(register) as u64).to_le_bytes());

        // `mov rcx, [rdx]`
        code.extend_from_slice(&[0x48, 0x8B, 0x0A]);
    }

    fn mov_absolute_rax(&self, code: &mut Vec<u8>, register: usize) {
        // `mov [absolute address], rax`
        code.extend_from_slice(&[0x48, 0xA3]);
        code.extend_from_slice(&(self.register(register) as u64).to_le_bytes());
    }

    fn load_integer_operands(&self, code: &mut Vec<u8>, lhs: OperandKind, rhs: OperandKind) {
        match lhs {
            // `mov rax, [absolute address]`
            OperandKind::Register(register) => self.mov_rax_register(code, register.0 as usize),

            // `mov rax, <immediate>`
            OperandKind::Immediate(value) => self.mov_rax_immediate(code, value),
        }

        match rhs {
            // Load the right-hand side through RDX so that RAX remains available for the left-hand side.
            // ```x86asm
            // mov rdx, <absolute address>
            // mov rcx, [rdx]
            // ```
            OperandKind::Register(register) => self.mov_rcx_register(code, register.0 as usize),

            // `mov rcx, <immediate>`
            OperandKind::Immediate(value) => self.mov_rcx_immediate(code, value),
        }
    }

    fn store_integer_result(&self, code: &mut Vec<u8>, destination: Register) {
        // `mov [absolute address], rax`
        self.mov_absolute_rax(code, destination.0 as usize);
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
        self.store_integer_result(&mut code, destination);

        Ok(code)
    }

    fn binary_compare(
        &self,
        instruction: &Instruction,
        setcc: &[u8; 3],
    ) -> Result<Vec<u8>, VMError> {
        let (destination, lhs, rhs) = instruction.expect3()?;
        let destination = destination.expect_register()?;
        let mut code = Vec::new();

        self.load_integer_operands(&mut code, lhs.kind, rhs.kind);

        // Compare lhs and rhs, setting flags for the `setcc` below.
        // `cmp rax, rcx`
        code.extend_from_slice(&[0x48, 0x39, 0xC8]);

        // Write 1 into AL if the condition holds, 0 otherwise.
        // The caller passes the `setcc al` encoding matching the opcode.
        code.extend_from_slice(setcc);

        // Zero-extend the 0/1 byte result to 64 bits, matching
        // `(condition) as u64` in the interpreter.
        // `movzx eax, al`
        code.extend_from_slice(&[0x0F, 0xB6, 0xC0]);

        self.store_integer_result(&mut code, destination);

        Ok(code)
    }

    fn movq_xmm0_rax(&self, code: &mut Vec<u8>) {
        // Move the f64 bit pattern from RAX into XMM0.
        // `movq xmm0, rax`
        code.extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC0]);
    }

    fn movq_xmm1_rcx(&self, code: &mut Vec<u8>) {
        // Move the f64 bit pattern from RCX into XMM1.
        // `movq xmm1, rcx`
        code.extend_from_slice(&[0x66, 0x48, 0x0F, 0x6E, 0xC9]);
    }

    fn movq_rax_xmm0(&self, code: &mut Vec<u8>) {
        // Move the computed f64 bit pattern back to RAX.
        // `movq rax, xmm0`
        code.extend_from_slice(&[0x66, 0x48, 0x0F, 0x7E, 0xC0]);
    }

    fn movq_rcx_xmm1(&self, code: &mut Vec<u8>) {
        // Move the second f64 bit pattern from XMM1 back into RCX.
        // `movq rcx, xmm1`
        code.extend_from_slice(&[0x66, 0x48, 0x0F, 0x7E, 0xC9]);
    }

    fn load_float_operands(&self, code: &mut Vec<u8>, lhs: OperandKind, rhs: OperandKind) {
        // Load the operands into the integer scratch registers first, then transfer their bits
        // into XMM registers. This keeps immediate and virtual-register operands uniform.
        match lhs {
            OperandKind::Register(register) => self.mov_rax_register(code, register.0 as usize),
            OperandKind::Immediate(value) => self.mov_rax_immediate(code, value),
        }
        self.movq_xmm0_rax(code);

        match rhs {
            OperandKind::Register(register) => self.mov_rcx_register(code, register.0 as usize),
            OperandKind::Immediate(value) => self.mov_rcx_immediate(code, value),
        }
        self.movq_xmm1_rcx(code);
    }

    fn store_float_result(&self, code: &mut Vec<u8>, destination: Register) {
        self.movq_rax_xmm0(code);
        // Store the f64 bit pattern from RAX into the destination register.
        // `mov [absolute address], rax`
        self.mov_absolute_rax(code, destination.0 as usize);
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

        // Compare the scalar double-precision values, setting ZF/PF/CF.
        // PF is set when unordered (either operand is NaN).
        // `ucomisd xmm0, xmm1`
        code.extend_from_slice(&[0x66, 0x0F, 0x2E, 0xC1]);

        // Write 1 into AL if the condition holds, 0 otherwise.
        // The caller passes the `setcc` sequence matching the opcode;
        // conditions affected by unordered results combine `setcc al`
        // with a parity-flag check in CL (RCX is free: both operands
        // already live in XMM registers).
        code.extend_from_slice(condition);

        // Zero-extend the 0/1 byte result to 64 bits, matching
        // `(condition) as u64` in the interpreter.
        // `movzx eax, al`
        code.extend_from_slice(&[0x0F, 0xB6, 0xC0]);

        // The result is an integer 0/1 in RAX (not an f64 in XMM0),
        // so it is stored directly instead of via `store_float_result`.
        // `mov [absolute address], rax`
        self.mov_absolute_rax(&mut code, destination.0 as usize);

        Ok(code)
    }
}
