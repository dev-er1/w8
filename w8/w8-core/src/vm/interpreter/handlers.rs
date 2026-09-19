// w8-core/src/vm/interpreter/handlers.rs
//
//! # Handlers
//!
//! Handlers are grouped by instruction “shapes”. For each shape,
//! specialized variants are generated per operand signature:
//! `r` — register, `i` — immediate (the letter order is the operand order).
use crate::vm::{
    VMCallDecision, WVM,
    err::{VMError, VMErrorKind},
    interpreter::{EXIT_MARKER, HandlerResult, Register, read_imm, read_reg, slot},
};

pub(crate) fn nop(_vm: &mut WVM, _p: *const u64, ip: usize) -> HandlerResult {
    Ok(ip + 1)
}

pub(crate) fn ret(vm: &mut WVM, _p: *const u64, _ip: usize) -> HandlerResult {
    let ip = vm
        .call_stack
        .pop()
        .ok_or_else(|| VMError::new(VMErrorKind::EmptyCallStack))?;
    Ok(ip)
}

#[inline]
fn ensure_nonzero_divisor(rhs: u64) -> Result<(), VMError> {
    if rhs == 0 {
        Err(VMError::new(VMErrorKind::DivisionByZero))
    } else {
        Ok(())
    }
}

// --------------- `VMCALL` ---------------
vmcall_variant!(vmcall_iii, imm, imm, imm);
vmcall_variant!(vmcall_rii, reg, imm, imm);
vmcall_variant!(vmcall_iri, imm, reg, imm);
vmcall_variant!(vmcall_iir, imm, imm, reg);
vmcall_variant!(vmcall_rri, reg, reg, imm);
vmcall_variant!(vmcall_rir, reg, imm, reg);
vmcall_variant!(vmcall_irr, imm, reg, reg);
vmcall_variant!(vmcall_rrr, reg, reg, reg);
// ---------------------------------------

// --------------- `MOVE` ---------------
move_variant!(move_ri, imm);
move_variant!(move_rr, reg);
// --------------------------------------

// --------------- `LOAD*` ---------------
load_variant!(load8_ri, load_u8, imm);
load_variant!(load8_rr, load_u8, reg);
load_variant!(load16_ri, load_u16, imm);
load_variant!(load16_rr, load_u16, reg);
load_variant!(load32_ri, load_u32, imm);
load_variant!(load32_rr, load_u32, reg);
load_variant!(load64_ri, load_u64, imm);
load_variant!(load64_rr, load_u64, reg);
// ---------------------------------------

// --------------- `STORE*` ---------------
store_variant!(store8_ii, store_u8, u8, imm, imm);
store_variant!(store8_ri, store_u8, u8, reg, imm);
store_variant!(store8_ir, store_u8, u8, imm, reg);
store_variant!(store8_rr, store_u8, u8, reg, reg);
store_variant!(store16_ii, store_u16, u16, imm, imm);
store_variant!(store16_ri, store_u16, u16, reg, imm);
store_variant!(store16_ir, store_u16, u16, imm, reg);
store_variant!(store16_rr, store_u16, u16, reg, reg);
store_variant!(store32_ii, store_u32, u32, imm, imm);
store_variant!(store32_ri, store_u32, u32, reg, imm);
store_variant!(store32_ir, store_u32, u32, imm, reg);
store_variant!(store32_rr, store_u32, u32, reg, reg);
store_variant!(store64_ii, store_u64, u64, imm, imm);
store_variant!(store64_ri, store_u64, u64, reg, imm);
store_variant!(store64_ir, store_u64, u64, imm, reg);
store_variant!(store64_rr, store_u64, u64, reg, reg);
// ----------------------------------------

// --------------- Integer operations ---------------
binops!(
    |l: u64, r: u64| l.wrapping_add(r),
    iadd_ii: imm imm,
    iadd_ri: reg imm,
    iadd_ir: imm reg,
    iadd_rr: reg reg,
);

binops!(
    |l: u64, r: u64| l.wrapping_sub(r),
    isub_ii: imm imm,
    isub_ri: reg imm,
    isub_ir: imm reg,
    isub_rr: reg reg,
);

binops!(
    |l: u64, r: u64| l.wrapping_mul(r),
    imul_ii: imm imm,
    imul_ri: reg imm,
    imul_ir: imm reg,
    imul_rr: reg reg,
);

unaries!(
    |v: u64| (v as i64).wrapping_neg() as u64,
    ineg_ri: imm,
    ineg_rr: reg,
);

binops!(
    |l: u64, r: u64| (l != r) as u64,
    ine_ii: imm imm,
    ine_ri: reg imm,
    ine_ir: imm reg,
    ine_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (l == r) as u64,
    ieq_ii: imm imm,
    ieq_ri: reg imm,
    ieq_ir: imm reg,
    ieq_rr: reg reg,
);
// --------------------------------------------------

// --------------- Signed integer operations ---------------
divisions!(
    |l: u64, r: u64| ((l as i64).wrapping_div(r as i64)) as u64,
    sdiv_ii: imm imm,
    sdiv_ri: reg imm,
    sdiv_ir: imm reg,
    sdiv_rr: reg reg,
);

divisions!(
    |l: u64, r: u64| ((l as i64).wrapping_rem(r as i64)) as u64,
    srem_ii: imm imm,
    srem_ri: reg imm,
    srem_ir: imm reg,
    srem_rr: reg reg,
);

binops!(
    |l: u64, r: u64| ((l as i64) < (r as i64)) as u64,
    slt_ii: imm imm,
    slt_ri: reg imm,
    slt_ir: imm reg,
    slt_rr: reg reg,
);

binops!(
    |l: u64, r: u64| ((l as i64) <= (r as i64)) as u64,
    sle_ii: imm imm,
    sle_ri: reg imm,
    sle_ir: imm reg,
    sle_rr: reg reg,
);

binops!(
    |l: u64, r: u64| ((l as i64) > (r as i64)) as u64,
    sgt_ii: imm imm,
    sgt_ri: reg imm,
    sgt_ir: imm reg,
    sgt_rr: reg reg,
);

binops!(
    |l: u64, r: u64| ((l as i64) >= (r as i64)) as u64,
    sge_ii: imm imm,
    sge_ri: reg imm,
    sge_ir: imm reg,
    sge_rr: reg reg,
);
// ---------------------------------------------------------

// --------------- Unsigned integer operations ---------------
divisions!(
    |l: u64, r: u64| l / r,
    udiv_ii: imm imm,
    udiv_ri: reg imm,
    udiv_ir: imm reg,
    udiv_rr: reg reg,
);

divisions!(
    |l: u64, r: u64| l % r,
    urem_ii: imm imm,
    urem_ri: reg imm,
    urem_ir: imm reg,
    urem_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (l < r) as u64,
    ult_ii: imm imm,
    ult_ri: reg imm,
    ult_ir: imm reg,
    ult_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (l <= r) as u64,
    ule_ii: imm imm,
    ule_ri: reg imm,
    ule_ir: imm reg,
    ule_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (l > r) as u64,
    ugt_ii: imm imm,
    ugt_ri: reg imm,
    ugt_ir: imm reg,
    ugt_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (l >= r) as u64,
    uge_ii: imm imm,
    uge_ri: reg imm,
    uge_ir: imm reg,
    uge_rr: reg reg,
);
// -----------------------------------------------------------

// --------------- Float operations ---------------

binops!(
    |l: u64, r: u64| (f64::from_bits(l) + f64::from_bits(r)).to_bits(),
    fadd_ii: imm imm,
    fadd_ri: reg imm,
    fadd_ir: imm reg,
    fadd_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) - f64::from_bits(r)).to_bits(),
    fsub_ii: imm imm,
    fsub_ri: reg imm,
    fsub_ir: imm reg,
    fsub_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) * f64::from_bits(r)).to_bits(),
    fmul_ii: imm imm,
    fmul_ri: reg imm,
    fmul_ir: imm reg,
    fmul_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) / f64::from_bits(r)).to_bits(),
    fdiv_ii: imm imm,
    fdiv_ri: reg imm,
    fdiv_ir: imm reg,
    fdiv_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) % f64::from_bits(r)).to_bits(),
    frem_ii: imm imm,
    frem_ri: reg imm,
    frem_ir: imm reg,
    frem_rr: reg reg,
);

unaries!(
    |v: u64| (-f64::from_bits(v)).to_bits(),
    fneg_ri: imm,
    fneg_rr: reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) == f64::from_bits(r)) as u64,
    feq_ii: imm imm,
    feq_ri: reg imm,
    feq_ir: imm reg,
    feq_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) != f64::from_bits(r)) as u64,
    fne_ii: imm imm,
    fne_ri: reg imm,
    fne_ir: imm reg,
    fne_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) < f64::from_bits(r)) as u64,
    flt_ii: imm imm,
    flt_ri: reg imm,
    flt_ir: imm reg,
    flt_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) <= f64::from_bits(r)) as u64,
    fle_ii: imm imm,
    fle_ri: reg imm,
    fle_ir: imm reg,
    fle_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) > f64::from_bits(r)) as u64,
    fgt_ii: imm imm,
    fgt_ri: reg imm,
    fgt_ir: imm reg,
    fgt_rr: reg reg,
);

binops!(
    |l: u64, r: u64| (f64::from_bits(l) >= f64::from_bits(r)) as u64,
    fge_ii: imm imm,
    fge_ri: reg imm,
    fge_ir: imm reg,
    fge_rr: reg reg,
);
// -------------------------------------------------

// --------------- Bitwise operations ---------------
binops!(
    |l: u64, r: u64| l & r,
    and_ii: imm imm,
    and_ri: reg imm,
    and_ir: imm reg,
    and_rr: reg reg,
);

binops!(
    |l: u64, r: u64| l | r,
    or_ii: imm imm,
    or_ri: reg imm,
    or_ir: imm reg,
    or_rr: reg reg,
);

binops!(
    |l: u64, r: u64| l ^ r,
    xor_ii: imm imm,
    xor_ri: reg imm,
    xor_ir: imm reg,
    xor_rr: reg reg,
);

unaries!(
    |v: u64| !v,
    not_ri: imm,
    not_rr: reg,
);
// ----------------------------------------------

// --------------- Shift operations ---------------
binops!(
    |l: u64, r: u64| l.wrapping_shl(r as u32),
    lsl_ii: imm imm,
    lsl_ri: reg imm,
    lsl_ir: imm reg,
    lsl_rr: reg reg,
);

binops!(
    |l: u64, r: u64| l.wrapping_shr(r as u32),
    lsr_ii: imm imm,
    lsr_ri: reg imm,
    lsr_ir: imm reg,
    lsr_rr: reg reg,
);

binops!(
    |l: u64, r: u64| ((l as i64).wrapping_shr(r as u32)) as u64,
    sar_ii: imm imm,
    sar_ri: reg imm,
    sar_ir: imm reg,
    sar_rr: reg reg,
);
// ------------------------------------------------

// --------------- Jumps ---------------
jmp_variant!(jmp_i, imm);
jmp_variant!(jmp_r, reg);
cond_variant!(jz_ii, imm, imm, |cond: u64| cond == 0);
cond_variant!(jz_ri, reg, imm, |cond: u64| cond == 0);
cond_variant!(jz_ir, imm, reg, |cond: u64| cond == 0);
cond_variant!(jz_rr, reg, reg, |cond: u64| cond == 0);
cond_variant!(jnz_ii, imm, imm, |cond: u64| cond != 0);
cond_variant!(jnz_ri, reg, imm, |cond: u64| cond != 0);
cond_variant!(jnz_ir, imm, reg, |cond: u64| cond != 0);
cond_variant!(jnz_rr, reg, reg, |cond: u64| cond != 0);
// -------------------------------------

// --------------- `CALL` ---------------
call_variant!(call_i, imm);
call_variant!(call_r, reg);
// --------------------------------------
