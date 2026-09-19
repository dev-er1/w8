// Tests for generating instructions from the AST.
use w8_core::isa::opcode::OperationCode;
use wdasm::codegen::err::CodegenErrorKind;

use super::*;

#[test]
fn program_without_labels_generates_as_is() {
    let program = codegen("MOVE R0, 42\nIADD R0, R1, 1").expect("valid program");

    assert_eq!(program.len(), 2);
    assert!(matches!(program[0].opcode, OperationCode::MOVE));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(42));
    assert!(matches!(program[1].opcode, OperationCode::IADD));
    assert_operand_eq(program[1].operand1, reg(0));
    assert_operand_eq(program[1].operand2, reg(1));
    assert_operand_eq(program[1].operand3, imm(1));
}

#[test]
fn label_points_to_next_instruction() {
    let program =
        codegen("begin:\nMOVE R0, 1\nJMP begin\nJMP end\nend:\nRET").expect("valid program");

    assert_eq!(program.len(), 4);
    // “begin” comes before MOVE (index 0), “end” — before RET (index 3).
    assert!(matches!(program[1].opcode, OperationCode::JMP));
    assert_operand_eq(program[1].operand1, imm(0));
    assert!(matches!(program[2].opcode, OperationCode::JMP));
    assert_operand_eq(program[2].operand1, imm(3));
}

#[test]
fn forward_jump_is_resolved() {
    let program = codegen("JMP loop\nNOP\nloop:\nNOP").expect("valid program");

    assert_eq!(program.len(), 3);
    assert!(matches!(program[0].opcode, OperationCode::JMP));
    assert_operand_eq(program[0].operand1, imm(2));
}

#[test]
fn backward_jump_is_resolved() {
    let program = codegen("loop:\nNOP\nJMP loop").expect("valid program");

    assert_eq!(program.len(), 2);
    assert!(matches!(program[1].opcode, OperationCode::JMP));
    assert_operand_eq(program[1].operand1, imm(0));
}

#[test]
fn call_resolves_to_instruction_index() {
    let program = codegen("CALL sub\nNOP\nsub:\nRET").expect("valid program");

    assert_eq!(program.len(), 3);
    assert!(matches!(program[0].opcode, OperationCode::CALL));
    assert_operand_eq(program[0].operand1, imm(2));
}

#[test]
fn jz_resolves_target_operand() {
    let program = codegen("JZ R0, skip\nNOP\nskip:\nNOP").expect("valid program");

    assert_eq!(program.len(), 3);
    assert!(matches!(program[0].opcode, OperationCode::JZ));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(2));
}

#[test]
fn jnz_resolves_target_operand() {
    let program = codegen("JNZ R0, skip\nNOP\nskip:\nNOP").expect("valid program");

    assert_eq!(program.len(), 3);
    assert!(matches!(program[0].opcode, OperationCode::JNZ));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(2));
}

#[test]
fn label_as_move_source() {
    let program = codegen("MOVE R0, here\nhere:\nNOP").expect("valid program");

    assert_eq!(program.len(), 2);
    assert!(matches!(program[0].opcode, OperationCode::MOVE));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(1));
}

#[test]
fn label_as_load_address() {
    let program = codegen("LOAD8 R0, data\ndata:\nNOP").expect("valid program");

    assert_eq!(program.len(), 2);
    assert!(matches!(program[0].opcode, OperationCode::LOAD8));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(1));
}

#[test]
fn several_labels_in_a_row() {
    let program = codegen("a:\nb:\nMOVE R0, 1").expect("valid program");

    assert_eq!(program.len(), 1);
    assert!(matches!(program[0].opcode, OperationCode::MOVE));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(1));
}

#[test]
fn label_after_last_instruction() {
    let program = codegen("JMP end\nend:").expect("valid program");

    assert_eq!(program.len(), 1);
    assert!(matches!(program[0].opcode, OperationCode::JMP));
    // The label is after the final — the jump targets an index equal to the program length
    // (such a jump terminates execution).
    assert_operand_eq(program[0].operand1, imm(1));
}

#[test]
fn zero_operand_instructions_keep_operands_empty() {
    let program = codegen("NOP\nNOP\nRET").expect("valid program");

    assert_eq!(program.len(), 3);
    assert_operand_none(program[0].operand1);
    assert_operand_none(program[1].operand2);
    assert_operand_none(program[2].operand3);
}

#[test]
fn label_in_third_operand_slot_is_resolved() {
    let program = codegen("IADD R0, R1, here\nhere:\nNOP").expect("valid program");

    assert_eq!(program.len(), 2);
    assert!(matches!(program[0].opcode, OperationCode::IADD));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, reg(1));
    assert_operand_eq(program[0].operand3, imm(1));
}

#[test]
fn multiple_references_to_the_same_label_all_resolve() {
    let program = codegen("JMP loop\nJMP loop\nloop:\nNOP").expect("valid program");

    assert_eq!(program.len(), 3);
    assert!(matches!(program[0].opcode, OperationCode::JMP));
    assert_operand_eq(program[0].operand1, imm(2));
    assert!(matches!(program[1].opcode, OperationCode::JMP));
    assert_operand_eq(program[1].operand1, imm(2));
}

#[test]
fn unused_labels_do_not_fail() {
    let program = codegen("unused:\nMOVE R0, 1").expect("valid program");

    assert_eq!(program.len(), 1);
    assert!(matches!(program[0].opcode, OperationCode::MOVE));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(1));
}

#[test]
fn label_only_program_generates_empty_program() {
    let program = codegen("start:").expect("valid program");

    assert_eq!(program.len(), 0);
    assert!(program.is_empty());
}

#[test]
fn empty_program_generates_empty_program() {
    let program = codegen("").expect("valid program");

    assert_eq!(program.len(), 0);
    assert!(program.is_empty());
}

#[test]
fn chained_jumps_resolve_in_order() {
    let program = codegen("a:\nJMP b\nb:\nJMP c\nc:\nRET").expect("valid program");

    assert_eq!(program.len(), 3);
    // a -> 0 (first instruction), b -> 1, c -> 2 (last).
    assert!(matches!(program[0].opcode, OperationCode::JMP));
    assert_operand_eq(program[0].operand1, imm(1));
    assert!(matches!(program[1].opcode, OperationCode::JMP));
    assert_operand_eq(program[1].operand1, imm(2));
}

#[test]
fn label_and_instruction_on_one_line_resolves() {
    let program = codegen("here: NOP\nJMP here").expect("valid program");

    assert_eq!(program.len(), 2);
    // The label on the same line before NOP — the jump target is index 0.
    assert!(matches!(program[1].opcode, OperationCode::JMP));
    assert_operand_eq(program[1].operand1, imm(0));
}

#[test]
fn local_label_resolves_within_its_scope() {
    let program = codegen("main:\nJMP *loop\n*loop:\nNOP").expect("valid program");

    assert_eq!(program.len(), 2);
    // “*loop” points to the instruction after its declaration (index 1).
    assert!(matches!(program[0].opcode, OperationCode::JMP));
    assert_operand_eq(program[0].operand1, imm(1));
}

#[test]
fn local_label_backward_jump_is_resolved() {
    let program = codegen("main:\n*loop:\nNOP\nJMP *loop").expect("valid program");

    assert_eq!(program.len(), 2);
    assert!(matches!(program[1].opcode, OperationCode::JMP));
    assert_operand_eq(program[1].operand1, imm(0));
}

#[test]
fn same_named_local_labels_in_different_scopes_are_independent() {
    let program = codegen("a:\n*x:\nNOP\nJMP *x\nb:\n*x:\nNOP\nJMP *x").expect("valid program");

    assert_eq!(program.len(), 4);
    // In scope “a” the reference goes to index 0, in scope “b” — to index 2.
    assert!(matches!(program[1].opcode, OperationCode::JMP));
    assert_operand_eq(program[1].operand1, imm(0));
    assert!(matches!(program[3].opcode, OperationCode::JMP));
    assert_operand_eq(program[3].operand1, imm(2));
}

#[test]
fn local_label_reference_in_second_operand_slot() {
    let program = codegen("main:\nJZ R0, *skip\n*skip:\nNOP").expect("valid program");

    assert_eq!(program.len(), 2);
    assert!(matches!(program[0].opcode, OperationCode::JZ));
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(1));
}

#[test]
fn local_label_does_not_leak_into_other_scopes() {
    let err = codegen("a:\n*x:\nNOP\nb:\nJMP *x").expect_err("cross-scope reference must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::UndefinedLabel { ref name } if name == "*x"
    ));
}

#[test]
fn define_value_becomes_an_immediate() {
    let program = codegen(".define WIDTH, 280\nMOVE R0, WIDTH").expect("valid program");

    assert_eq!(program.len(), 1);
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(280));
}

#[test]
fn define_register_alias_becomes_a_register() {
    let program = codegen(".define RAX, R0\nMOVE RAX, R1").expect("valid program");

    assert_eq!(program.len(), 1);
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, reg(1));
}

#[test]
fn define_is_substituted_in_any_operand_slot() {
    let program = codegen(".define ONE, 1\nIADD R0, R1, ONE").expect("valid program");

    assert_eq!(program.len(), 1);
    assert_operand_eq(program[0].operand3, imm(1));
}

#[test]
fn define_does_not_emit_instructions() {
    let program = codegen(".define X, 5\n.define Y, 6\nNOP").expect("valid program");

    assert_eq!(program.len(), 1);
    assert!(matches!(program[0].opcode, OperationCode::NOP));
}

#[test]
fn define_used_before_declaration_is_an_undefined_label() {
    // The substitution happens in source order: a use before the
    // declaration is not seen and becomes a label reference.
    let err = codegen("MOVE R0, LATE\n.define LATE, 5").expect_err("late define must fail");

    assert!(matches!(
        err.kind,
        CodegenErrorKind::UndefinedLabel { ref name } if name == "LATE"
    ));
}

#[test]
fn extreme_registers_are_preserved() {
    let program = codegen("MOVE R254, 42\nIADD R0, R254, R200\nNOT R1, R2").expect("valid program");

    assert_eq!(program.len(), 3);
    assert_operand_eq(program[0].operand1, reg(254));
    assert_operand_eq(program[0].operand2, imm(42));
    assert_operand_eq(program[1].operand1, reg(0));
    assert_operand_eq(program[1].operand2, reg(254));
    assert_operand_eq(program[1].operand3, reg(200));
    assert_operand_eq(program[2].operand1, reg(1));
    assert_operand_eq(program[2].operand2, reg(2));
}

#[test]
fn zero_immediate_is_kept_as_immediate() {
    let program = codegen("MOVE R0, 0").expect("valid program");

    assert_eq!(program.len(), 1);
    assert_operand_eq(program[0].operand1, reg(0));
    assert_operand_eq(program[0].operand2, imm(0));
}

#[test]
fn jump_to_label_at_last_instruction() {
    let program = codegen("JMP last\nNOP\nlast:\nNOP").expect("valid program");

    assert_eq!(program.len(), 3);
    assert!(matches!(program[0].opcode, OperationCode::JMP));
    assert_operand_eq(program[0].operand1, imm(2));
}

#[test]
fn labels_differ_by_case() {
    let program = codegen("A:\nMOVE R0, 1\na:\nJMP A").expect("valid program");

    assert_eq!(program.len(), 2);
    // “A” and “a” are different labels: jumping on “A” goes to index 0.
    assert!(matches!(program[1].opcode, OperationCode::JMP));
    assert_operand_eq(program[1].operand1, imm(0));
}

#[test]
fn long_program_resolves_distant_labels() {
    let mut src = String::from("start:\nJMP mid\n");
    for _ in 0..10 {
        src.push_str("NOP\n");
    }
    src.push_str("mid:\nJMP start\n");
    for _ in 0..10 {
        src.push_str("NOP\n");
    }

    let program = codegen(&src).expect("valid program");

    assert_eq!(program.len(), 22);
    // “start” -> 0 (first instruction), “mid” -> 11 (before “JMP start”).
    assert!(matches!(program[0].opcode, OperationCode::JMP));
    assert_operand_eq(program[0].operand1, imm(11));
    assert!(matches!(program[11].opcode, OperationCode::JMP));
    assert_operand_eq(program[11].operand1, imm(0));
}

#[test]
fn align_pads_with_nops_to_a_multiple() {
    let program = codegen("NOP\n.align 4").expect("valid program");

    assert_eq!(program.len(), 4);
    for instr in &program {
        assert!(matches!(instr.opcode, OperationCode::NOP));
        assert_operand_none(instr.operand1);
    }
}

#[test]
fn align_when_already_aligned_does_nothing() {
    let program = codegen(".align 4\nNOP\n.align 2").expect("valid program");

    // 0 -> .align 4 (no padding), 1 -> NOP, then .align 2 pads by 1.
    assert_eq!(program.len(), 2);
    for instr in &program {
        assert!(matches!(instr.opcode, OperationCode::NOP));
    }
}

#[test]
fn align_one_is_a_noop() {
    let program = codegen("NOP\n.align 1").expect("valid program");

    assert_eq!(program.len(), 1);
    assert!(matches!(program[0].opcode, OperationCode::NOP));
}

#[test]
fn align_in_the_middle_of_the_program() {
    let program = codegen("MOVE R0, 1\nMOVE R1, 2\n.align 8\nRET").expect("valid program");

    assert_eq!(program.len(), 9);
    assert!(matches!(program[0].opcode, OperationCode::MOVE));
    assert!(matches!(program[1].opcode, OperationCode::MOVE));
    // The padding positions 2..8 are NOPs.
    for instr in &program[2..8] {
        assert!(matches!(instr.opcode, OperationCode::NOP));
    }
    assert!(matches!(program[8].opcode, OperationCode::RET));
}

#[test]
fn align_affects_label_resolution() {
    let program = codegen("JMP target\n.align 4\ntarget:\nNOP").expect("valid program");

    assert_eq!(program.len(), 5);
    // The label after `.align` points past the padding.
    assert!(matches!(program[0].opcode, OperationCode::JMP));
    assert_operand_eq(program[0].operand1, imm(4));
}

#[test]
fn label_before_align_points_to_the_padding() {
    let program = codegen("here: NOP\n.align 4\nJMP here").expect("valid program");

    assert_eq!(program.len(), 5);
    // A label before the directive points to the instruction before the padding.
    assert!(matches!(program[4].opcode, OperationCode::JMP));
    assert_operand_eq(program[4].operand1, imm(0));
    // The padding fills positions 1..4.
    for instr in &program[1..4] {
        assert!(matches!(instr.opcode, OperationCode::NOP));
    }
}

#[test]
fn align_with_big_multiple_pads_a_lot() {
    let program = codegen("NOP\n.align 256").expect("valid program");

    assert_eq!(program.len(), 256);
    for instr in &program {
        assert!(matches!(instr.opcode, OperationCode::NOP));
    }
}

#[test]
fn without_minversion_the_version_is_none() {
    let result = codegen_result("NOP").expect("valid program");

    assert_eq!(result.instructions.len(), 1);
    assert!(result.min_version.is_none());
}
