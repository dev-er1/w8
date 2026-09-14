//! # W8 bytecode executor
//!
//! This module implements the W8 instruction executor —
//! based on *Direct Threading* (direct threaded dispatch).
//!
//! ## The idea
//!
//! The program is encoded **once** into a flat array of [`u64`]:
//!
//! 4 slots of 8 bytes per instruction.
//!
//! Slot 0 is the header: the **handler address** of this instruction, chosen
//! by the opcode and the operand kinds. “Kinds” are one bit per operand.
//! Handlers are specialized by signature: one for each valid combination
//! of operand kinds. The handler has no branches by operand kind — the
//! branching is eliminated already at encoding time.
//!
//! The jump table is used **only at the encoding stage**:
//! the handler addresses are placed directly into the program stream, so
//! in the hot loop there is no table indexing, no dispatch index
//! computation, and no table bounds check.
//!
//! Each operand is “flattened” into a number:
//! - a register — into the register number (the “register” bit in the header);
//! - an immediate — as is (the bit cleared);
//! - a missing operand — into 0 (the bit cleared).
//!
//! The operand count and the mandatory operand types (destinations must be
//! registers) are checked **once** at encoding, before execution starts.
//!
//! The entry point is [`WVM::run`].
#[macro_use]
pub mod macros;
pub mod handlers;
pub mod jumptable;

use crate::{
    isa::{
        instruction::Instruction,
        opcode::OperationCode,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::{
        VMCallDecision, WVM,
        err::{VMError, VMErrorKind},
        interpreter::jumptable::build_jump_table,
    },
};

/// How many slots one instruction occupies in the encoded program.
const SLOTS: usize = 4;

/// The number of W8 opcodes.
const OPCODE_COUNT: usize = OperationCode::VMCALL as usize + 1;

/// The jump table: `index = opcode * 8 + operand kinds`.
///
/// Used **only at the encoding stage** ([`encode`]) to put the address
/// of the required handler into the program stream. The hot loop does not
/// touch the table.
static JUMP_TABLE: [Handler; TABLE_LEN] = build_jump_table();

/// The number of entries in the jump table: 8 signatures per opcode.
const TABLE_LEN: usize = OPCODE_COUNT * 8;

/// The terminal value for the next `ip`: ends execution, since
/// it is `>= instruction_count`.
const EXIT_MARKER: usize = usize::MAX;

/// The handler result: the index of the next instruction.
type HandlerResult = Result<usize, VMError>;

/// The handler function of a single instruction.
///
/// Receives the VM, a pointer to the operand slots of the current instruction
/// (slot `0` is operand 1, slot `1` is operand 2, slot `2` is operand 3),
/// and the index of the current instruction. Returns the index of the next instruction.
///
/// ## Safety
///
/// The pointer points to the operands of the instruction with index `ip`
/// in the encoded program (guaranteed by the invariant
/// `0 <= ip < instruction_count` in [`WVM::run`]);
/// the handler reads only slots `0..SLOTS - 1` relative to it,
/// i.e. strictly within the instruction.
type Handler = unsafe fn(&mut WVM, *const u64, usize) -> HandlerResult;

/// Reads an operand slot via the pointer.
#[inline(always)]
#[allow(clippy::not_unsafe_ptr_arg_deref)]
pub fn slot(p: *const u64, n: usize) -> u64 {
    // SAFETY: see `Handler` — the slots are within the instruction.
    unsafe { *p.add(n) }
}

/// Reads the value of a register operand (the slot holds the register number).
#[inline(always)]
pub fn read_reg(vm: &WVM, p: *const u64, n: usize) -> u64 {
    vm.registers[Register(slot(p, n) as u8)]
}

/// Reads the value of an immediate operand (the slot holds the value).
#[inline(always)]
pub fn read_imm(p: *const u64, n: usize) -> u64 {
    slot(p, n)
}

impl WVM {
    /// Run the program.
    pub fn interpretate(&mut self) -> Result<(), VMError> {
        // A fresh run resets the exit code from a previous one.
        self.exit_code = None;

        // Encode the program.
        let code = encode(&self.program)?;
        let instruction_count = self.program.len();

        if instruction_count == 0 {
            return Ok(());
        }

        let mut ip = 0usize;

        // Loop invariant: `0 <= ip < instruction_count`, so reading
        // the instruction header and the operand slots (through the handler)
        // never goes out of bounds of `code`.
        loop {
            // SAFETY: `ip < instruction_count`, see the invariant above.
            let base = unsafe { code.as_ptr().add(ip * SLOTS) };

            // SAFETY: the header is the instruction's first slot (within bounds);
            // it stores the handler address, see [`encode`].
            let handler: Handler = unsafe { std::mem::transmute(*base) };

            // SAFETY: the handler reads only the operand slots
            // of the current instruction, see [`Handler`].
            let next = unsafe { handler(self, base.add(1), ip) }?;

            // A `VMCALL` with an `Exit` decision returns [`EXIT_MARKER`];
            // going past the end of the program also ends execution.
            if next >= instruction_count {
                return Ok(());
            }
            ip = next;
        }
    }

    /// Runs the VM program with a host dispatcher for `VMCALL`.
    ///
    /// The dispatcher is stored in [`WVM::dispatch`] for the duration of
    /// the run ([`WVM::interpretate`]).
    pub fn interpretate_with(
        &mut self,
        dispatch: impl FnMut(&mut WVM, u64, u64, u64) -> VMCallDecision + 'static,
    ) -> Result<(), VMError> {
        self.dispatch = Some(Box::new(dispatch));
        self.interpretate()
    }
}

// --- Helpers ---

// The expected operand count and the slots that must be registers.
//
// Slots are numbered from `1` (slot `0` is the header). Slots not in
// `register_slots` can be either a register or an immediate.
fn operand_pattern(opcode: OperationCode) -> (u8, &'static [usize]) {
    use OperationCode::*;

    match opcode {
        NOP | RET => (0, &[]),
        JMP | CALL => (1, &[]),
        JZ | JNZ => (2, &[]),
        MOVE | LOAD8 | LOAD16 | LOAD32 | LOAD64 => (2, &[1]),
        STORE8 | STORE16 | STORE32 | STORE64 => (2, &[]),
        INEG | FNEG | NOT => (2, &[1]),
        VMCALL => (3, &[]),
        IADD | ISUB | IMUL | SDIV | UDIV | SREM | UREM => (3, &[1]),
        FADD | FSUB | FMUL | FDIV | FREM => (3, &[1]),
        AND | OR | XOR | LSL | LSR | SAR => (3, &[1]),
        IEQ | INE | SLT | SLE | SGT | SGE | ULT | ULE | UGT | UGE => (3, &[1]),
        FEQ | FNE | FLT | FLE | FGT | FGE => (3, &[1]),
    }
}

// Whether the operand is a register.
fn is_register(operand: &Option<Operand>) -> bool {
    matches!(
        operand,
        Some(Operand {
            kind: OperandKind::Register(_),
            ..
        })
    )
}

// Encodes a program into a flat array of `u64`.
//
// Each instruction occupies [`SLOTS`] slots:
// `[header, operand1, operand2, operand3]`. The header holds the address
// of the handler for this instruction (chosen by the opcode and operand kinds,
// see [`build_jump_table`]).
//
// On encoding, the operand count and the required operand types
// are checked (see [`operand_pattern`]).
fn encode(program: &[Instruction]) -> Result<Vec<u64>, VMError> {
    let mut code = Vec::with_capacity(program.len() * SLOTS);

    for instr in program {
        let count = instr.operand_count();
        let (expected, register_slots) = operand_pattern(instr.opcode);

        if count != expected as usize {
            return Err(VMError::new(VMErrorKind::IncorrectNumberOfOperands {
                expected,
                got: count as u8,
            }));
        }

        for &slot in register_slots {
            let operand = [&instr.operand1, &instr.operand2, &instr.operand3][slot - 1];
            if !is_register(operand) {
                return Err(VMError::new(VMErrorKind::IncorrectTypeOfOperand {
                    expected: OperandKind::Register(Register(0)),
                    got: operand.map(|o| o.kind).unwrap_or(OperandKind::Immediate(0)),
                }));
            }
        }

        // The register numbers themselves: W8 has 255 registers (0-254).
        for operand in [&instr.operand1, &instr.operand2, &instr.operand3]
            .into_iter()
            .flatten()
        {
            if let OperandKind::Register(register) = operand.kind
                && register.0 > Register::MAX_INDEX
            {
                return Err(VMError::new(VMErrorKind::InvalidRegister {
                    got: register.0,
                }));
            }
        }

        let kinds = (is_register(&instr.operand1) as u64)
            | (is_register(&instr.operand2) as u64) << 1
            | (is_register(&instr.operand3) as u64) << 2;

        // Dispatch: the handler address itself is placed into the header.
        // The jump table is used only at the encoding stage —
        // in the hot loop it is absent.
        let handler = JUMP_TABLE[instr.opcode as usize * 8 + kinds as usize];

        // SAFETY: `Handler` is a function pointer; on the target platforms
        // of W8 (64-bit) it is representable as `u64`.
        code.push(handler as *const () as u64);
        code.push(flatten(instr.operand1));
        code.push(flatten(instr.operand2));
        code.push(flatten(instr.operand3));
    }

    Ok(code)
}

// “Flattens” an operand into a number.
fn flatten(operand: Option<Operand>) -> u64 {
    match operand {
        Some(Operand {
            kind: OperandKind::Register(r),
        }) => u64::from(r.0),
        Some(Operand {
            kind: OperandKind::Immediate(v),
        }) => v,
        None => 0,
    }
}
