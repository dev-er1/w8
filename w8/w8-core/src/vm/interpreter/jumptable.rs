// w8-core/src/vm/interpreter/jumptable.rs
use crate::{
    isa::opcode::OperationCode,
    vm::{
        WVM,
        interpreter::{Handler, HandlerResult, TABLE_LEN, handlers::*},
    },
};

fn invalid_signature(_vm: &mut WVM, _p: *const u64, _ip: usize) -> HandlerResult {
    unreachable!("jump table: invalid operand signature reached")
}

macro_rules! register {
    ($table:ident, $opcode:expr, $kinds:expr, $handler:ident) => {
        $table[$opcode as usize * 8 + $kinds] = $handler;
    };
}

macro_rules! register_binary {
    ($table:ident, $opcode:expr, $ii:ident, $ri:ident, $ir:ident, $rr:ident) => {
        register!($table, $opcode, 1, $ii);
        register!($table, $opcode, 3, $ri);
        register!($table, $opcode, 5, $ir);
        register!($table, $opcode, 7, $rr);
    };
}

macro_rules! register_store {
    ($table:ident, $opcode:expr, $ii:ident, $ri:ident, $ir:ident, $rr:ident) => {
        register!($table, $opcode, 0, $ii);
        register!($table, $opcode, 1, $ri);
        register!($table, $opcode, 2, $ir);
        register!($table, $opcode, 3, $rr);
    };
}

macro_rules! register_rr {
    ($table:ident, $opcode:expr, $ri:ident, $rr:ident) => {
        register!($table, $opcode, 1, $ri);
        register!($table, $opcode, 3, $rr);
    };
}

macro_rules! register_vmcall {
    ($table:ident, $opcode:expr, $iii:ident, $rii:ident, $iri:ident, $iir:ident, $rri:ident, $rir:ident, $irr:ident, $rrr:ident) => {
        register!($table, $opcode, 0, $iii);
        register!($table, $opcode, 1, $rii);
        register!($table, $opcode, 2, $iri);
        register!($table, $opcode, 3, $rri);
        register!($table, $opcode, 4, $iir);
        register!($table, $opcode, 5, $rir);
        register!($table, $opcode, 6, $irr);
        register!($table, $opcode, 7, $rrr);
    };
}

macro_rules! register_cond {
    ($table:ident, $opcode:expr, $ii:ident, $ri:ident, $ir:ident, $rr:ident) => {
        register!($table, $opcode, 0, $ii);
        register!($table, $opcode, 1, $ri);
        register!($table, $opcode, 2, $ir);
        register!($table, $opcode, 3, $rr);
    };
}

/// Builds the jump table: `index = opcode * 8 + operand kinds`.
///
/// For each opcode, only valid signatures are registered
/// (destinations are always registers); the remaining slots are the
/// [`invalid_signature`] stub.
pub(crate) const fn build_jump_table() -> [Handler; TABLE_LEN] {
    let mut table = [invalid_signature as Handler; TABLE_LEN];

    register!(table, OperationCode::NOP, 0, nop);

    register_rr!(table, OperationCode::MOVE, move_ri, move_rr);

    register_rr!(table, OperationCode::LOAD8, load8_ri, load8_rr);
    register_rr!(table, OperationCode::LOAD16, load16_ri, load16_rr);
    register_rr!(table, OperationCode::LOAD32, load32_ri, load32_rr);
    register_rr!(table, OperationCode::LOAD64, load64_ri, load64_rr);

    register_store!(
        table,
        OperationCode::STORE8,
        store8_ii,
        store8_ri,
        store8_ir,
        store8_rr
    );
    register_store!(
        table,
        OperationCode::STORE16,
        store16_ii,
        store16_ri,
        store16_ir,
        store16_rr
    );
    register_store!(
        table,
        OperationCode::STORE32,
        store32_ii,
        store32_ri,
        store32_ir,
        store32_rr
    );
    register_store!(
        table,
        OperationCode::STORE64,
        store64_ii,
        store64_ri,
        store64_ir,
        store64_rr
    );

    register_binary!(
        table,
        OperationCode::IADD,
        iadd_ii,
        iadd_ri,
        iadd_ir,
        iadd_rr
    );
    register_binary!(
        table,
        OperationCode::ISUB,
        isub_ii,
        isub_ri,
        isub_ir,
        isub_rr
    );
    register_binary!(
        table,
        OperationCode::IMUL,
        imul_ii,
        imul_ri,
        imul_ir,
        imul_rr
    );

    register_binary!(
        table,
        OperationCode::SDIV,
        sdiv_ii,
        sdiv_ri,
        sdiv_ir,
        sdiv_rr
    );
    register_binary!(
        table,
        OperationCode::UDIV,
        udiv_ii,
        udiv_ri,
        udiv_ir,
        udiv_rr
    );
    register_binary!(
        table,
        OperationCode::SREM,
        srem_ii,
        srem_ri,
        srem_ir,
        srem_rr
    );
    register_binary!(
        table,
        OperationCode::UREM,
        urem_ii,
        urem_ri,
        urem_ir,
        urem_rr
    );

    register_rr!(table, OperationCode::INEG, ineg_ri, ineg_rr);

    register_binary!(
        table,
        OperationCode::FADD,
        fadd_ii,
        fadd_ri,
        fadd_ir,
        fadd_rr
    );
    register_binary!(
        table,
        OperationCode::FSUB,
        fsub_ii,
        fsub_ri,
        fsub_ir,
        fsub_rr
    );
    register_binary!(
        table,
        OperationCode::FMUL,
        fmul_ii,
        fmul_ri,
        fmul_ir,
        fmul_rr
    );
    register_binary!(
        table,
        OperationCode::FDIV,
        fdiv_ii,
        fdiv_ri,
        fdiv_ir,
        fdiv_rr
    );
    register_binary!(
        table,
        OperationCode::FREM,
        frem_ii,
        frem_ri,
        frem_ir,
        frem_rr
    );

    register_rr!(table, OperationCode::FNEG, fneg_ri, fneg_rr);

    register_binary!(table, OperationCode::AND, and_ii, and_ri, and_ir, and_rr);
    register_binary!(table, OperationCode::OR, or_ii, or_ri, or_ir, or_rr);
    register_binary!(table, OperationCode::XOR, xor_ii, xor_ri, xor_ir, xor_rr);

    register_rr!(table, OperationCode::NOT, not_ri, not_rr);

    register_binary!(table, OperationCode::LSL, lsl_ii, lsl_ri, lsl_ir, lsl_rr);
    register_binary!(table, OperationCode::LSR, lsr_ii, lsr_ri, lsr_ir, lsr_rr);
    register_binary!(table, OperationCode::SAR, sar_ii, sar_ri, sar_ir, sar_rr);

    register_binary!(table, OperationCode::IEQ, ieq_ii, ieq_ri, ieq_ir, ieq_rr);
    register_binary!(table, OperationCode::INE, ine_ii, ine_ri, ine_ir, ine_rr);
    register_binary!(table, OperationCode::SLT, slt_ii, slt_ri, slt_ir, slt_rr);
    register_binary!(table, OperationCode::SLE, sle_ii, sle_ri, sle_ir, sle_rr);
    register_binary!(table, OperationCode::SGT, sgt_ii, sgt_ri, sgt_ir, sgt_rr);
    register_binary!(table, OperationCode::SGE, sge_ii, sge_ri, sge_ir, sge_rr);
    register_binary!(table, OperationCode::ULT, ult_ii, ult_ri, ult_ir, ult_rr);
    register_binary!(table, OperationCode::ULE, ule_ii, ule_ri, ule_ir, ule_rr);
    register_binary!(table, OperationCode::UGT, ugt_ii, ugt_ri, ugt_ir, ugt_rr);
    register_binary!(table, OperationCode::UGE, uge_ii, uge_ri, uge_ir, uge_rr);

    register_binary!(table, OperationCode::FEQ, feq_ii, feq_ri, feq_ir, feq_rr);
    register_binary!(table, OperationCode::FNE, fne_ii, fne_ri, fne_ir, fne_rr);
    register_binary!(table, OperationCode::FLT, flt_ii, flt_ri, flt_ir, flt_rr);
    register_binary!(table, OperationCode::FLE, fle_ii, fle_ri, fle_ir, fle_rr);
    register_binary!(table, OperationCode::FGT, fgt_ii, fgt_ri, fgt_ir, fgt_rr);
    register_binary!(table, OperationCode::FGE, fge_ii, fge_ri, fge_ir, fge_rr);

    register!(table, OperationCode::JMP, 0, jmp_i);
    register!(table, OperationCode::JMP, 1, jmp_r);

    register_cond!(table, OperationCode::JZ, jz_ii, jz_ri, jz_ir, jz_rr);
    register_cond!(table, OperationCode::JNZ, jnz_ii, jnz_ri, jnz_ir, jnz_rr);

    register!(table, OperationCode::CALL, 0, call_i);
    register!(table, OperationCode::CALL, 1, call_r);

    register!(table, OperationCode::RET, 0, ret);

    register_vmcall!(
        table,
        OperationCode::VMCALL,
        vmcall_iii,
        vmcall_rii,
        vmcall_iri,
        vmcall_iir,
        vmcall_rri,
        vmcall_rir,
        vmcall_irr,
        vmcall_rrr
    );

    table
}
