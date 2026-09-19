// Tests for parsing labels, instructions, and directives.
use w8_core::isa::opcode::OperationCode;
use wdasm::lexer::directive::Directive;
use wdasm::parser::ast::{Operand, Statement};

use super::*;

#[test]
fn empty_program_is_an_empty_ast() {
    let (ast, errors) = parse("");

    assert!(errors.is_empty());
    assert!(ast.program.is_empty());
}

#[test]
fn blank_lines_and_comments_are_skipped() {
    let (ast, errors) = parse("\n\n; только комментарий\n");

    assert!(errors.is_empty());
    assert!(ast.program.is_empty());
}

#[test]
fn label_only_statement() {
    let (ast, errors) = parse("main:");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 1);
    assert!(matches!(
        ast.program[0],
        Statement::Label {
            position,
            ..
        } if position.start == 0 && position.end == 5
    ));
}

#[test]
fn label_position_covers_name_and_colon() {
    let (ast, errors) = parse(" main:");

    assert!(errors.is_empty());
    match &ast.program[0] {
        Statement::Label { position, .. } => {
            assert_eq!(position.start, 1);
            assert_eq!(position.end, 6);
        }
        _ => panic!("expected a label"),
    }
}

#[test]
fn label_and_instruction_on_one_line() {
    let (ast, errors) = parse("main: MOVE R0, 42");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 2);
    assert!(matches!(ast.program[0], Statement::Label { .. }));
    match &ast.program[1] {
        Statement::Instruction { instruction, .. } => {
            assert!(matches!(instruction.opcode, OperationCode::MOVE));
            assert!(matches!(
                instruction.operand1,
                Some(Operand::Register(r)) if r.0 == 0
            ));
            assert!(matches!(instruction.operand2, Some(Operand::Immediate(42))));
        }
        _ => panic!("expected an instruction"),
    }
}

#[test]
fn two_labels_and_instruction_on_one_line() {
    let (ast, errors) = parse("a: b: NOP");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 3);
    assert!(matches!(ast.program[0], Statement::Label { .. }));
    assert!(matches!(ast.program[1], Statement::Label { .. }));
    assert!(matches!(ast.program[2], Statement::Instruction { .. }));
}

#[test]
fn register_operands() {
    let (ast, errors) = parse("MOVE R0, R1");

    assert!(errors.is_empty());
    let instruction = first_instr(&ast);
    assert!(matches!(instruction.operand1, Some(Operand::Register(r)) if r.0 == 0));
    assert!(matches!(instruction.operand2, Some(Operand::Register(r)) if r.0 == 1));
    assert!(instruction.operand3.is_none());
}

#[test]
fn integer_operands() {
    let (ast, errors) = parse("MOVE R0, 42");

    assert!(errors.is_empty());
    let instruction = first_instr(&ast);
    assert!(matches!(instruction.operand1, Some(Operand::Register(r)) if r.0 == 0));
    assert!(matches!(instruction.operand2, Some(Operand::Immediate(42))));
}

#[test]
fn negative_integer_wraps_to_u64() {
    let (ast, errors) = parse("MOVE R0, -1");

    assert!(errors.is_empty());
    let instruction = first_instr(&ast);
    assert!(matches!(
        instruction.operand2,
        Some(Operand::Immediate(v)) if v == u64::MAX
    ));
}

#[test]
fn float_becomes_bit_pattern() {
    let (ast, errors) = parse("FADD R0, 1.0, 2.0");

    assert!(errors.is_empty());
    let instruction = first_instr(&ast);
    assert!(matches!(
        instruction.operand2,
        Some(Operand::Immediate(v)) if v == 1.0f64.to_bits()
    ));
    assert!(matches!(
        instruction.operand3,
        Some(Operand::Immediate(v)) if v == 2.0f64.to_bits()
    ));
}

#[test]
fn label_reference_is_kept_in_ast() {
    let (ast, errors) = parse("JMP loop\nloop: NOP");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 3);

    match (&ast.program[0], &ast.program[1]) {
        (Statement::Instruction { instruction, .. }, Statement::Label { name, .. }) => {
            assert!(matches!(instruction.operand1, Some(Operand::Label(id)) if id == *name));
        }
        _ => panic!("expected an instruction and a label"),
    }
}

#[test]
fn local_label_declaration_is_marked_local() {
    let (ast, errors) = parse("main:\n*loop:");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 2);

    match (&ast.program[0], &ast.program[1]) {
        (
            Statement::Label {
                name: global,
                local: false,
                ..
            },
            Statement::Label {
                name: local,
                local: true,
                ..
            },
        ) => {
            assert_ne!(global, local);
        }
        _ => panic!("expected a global and a local label"),
    }
}

#[test]
fn local_label_reference_is_kept_in_ast() {
    let (ast, errors) = parse("main:\nJMP *loop\n*loop: NOP");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 4);

    match (&ast.program[0], &ast.program[1], &ast.program[2]) {
        (
            Statement::Label { .. },
            Statement::Instruction { instruction, .. },
            Statement::Label {
                name, local: true, ..
            },
        ) => {
            assert!(matches!(instruction.operand1, Some(Operand::LocalLabel(id)) if id == *name));
        }
        _ => panic!("expected a label, an instruction, and a local label"),
    }
}

#[test]
fn local_label_in_third_operand_slot() {
    let (ast, errors) = parse("IADD R0, R1, *here");

    assert!(errors.is_empty());
    let instruction = first_instr(&ast);
    assert!(matches!(instruction.operand3, Some(Operand::LocalLabel(_))));
}

#[test]
fn define_is_substituted_in_instruction_operands() {
    let (ast, errors) = parse(".define WIDTH, 280\nMOVE R0, WIDTH");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 2);

    let instruction = match &ast.program[1] {
        Statement::Instruction { instruction, .. } => instruction,
        _ => panic!("expected an instruction"),
    };
    assert!(matches!(
        instruction.operand2,
        Some(Operand::Immediate(280))
    ));
}

#[test]
fn define_float_value_keeps_the_bit_pattern() {
    let (ast, errors) = parse(".define HALF, 0.5\nFADD R0, R1, HALF");

    assert!(errors.is_empty());
    let instruction = match &ast.program[1] {
        Statement::Instruction { instruction, .. } => instruction,
        _ => panic!("expected an instruction"),
    };
    assert!(matches!(
        instruction.operand3,
        Some(Operand::Immediate(v)) if v == 0.5f64.to_bits()
    ));
}

#[test]
fn the_last_define_of_a_name_wins() {
    let (ast, errors) = parse(".define X, 1\n.define X, 2\nMOVE R0, X");

    assert!(errors.is_empty());
    let instruction = match &ast.program[2] {
        Statement::Instruction { instruction, .. } => instruction,
        _ => panic!("expected an instruction"),
    };
    assert!(matches!(instruction.operand2, Some(Operand::Immediate(2))));
}

#[test]
fn define_register_alias_is_substituted_in_operands() {
    let (ast, errors) = parse(".define RAX, R0\nMOVE RAX, R1");

    assert!(errors.is_empty());
    let instruction = match &ast.program[1] {
        Statement::Instruction { instruction, .. } => instruction,
        _ => panic!("expected an instruction"),
    };
    assert!(matches!(
        instruction.operand1,
        Some(Operand::Register(r)) if r.0 == 0
    ));
    assert!(matches!(
        instruction.operand2,
        Some(Operand::Register(r)) if r.0 == 1
    ));
}

#[test]
fn define_register_alias_in_third_operand_slot() {
    let (ast, errors) = parse(".define B, R2\nIADD R0, R1, B");

    assert!(errors.is_empty());
    let instruction = match &ast.program[1] {
        Statement::Instruction { instruction, .. } => instruction,
        _ => panic!("expected an instruction"),
    };
    assert!(matches!(
        instruction.operand3,
        Some(Operand::Register(r)) if r.0 == 2
    ));
}

#[test]
fn define_register_alias_as_the_destination() {
    let (ast, errors) = parse(".define ACC, R7\nIADD ACC, R0, R1");

    assert!(errors.is_empty());
    let instruction = match &ast.program[1] {
        Statement::Instruction { instruction, .. } => instruction,
        _ => panic!("expected an instruction"),
    };

    assert!(matches!(
        instruction.operand1,
        Some(Operand::Register(r)) if r.0 == 7
    ));
}

#[test]
fn define_does_not_turn_a_label_into_a_value() {
    let (ast, errors) = parse("MOVE R0, SOMEWHERE");

    assert!(errors.is_empty());
    let instruction = first_instr(&ast);
    assert!(matches!(instruction.operand2, Some(Operand::Label(_))));
}

#[test]
fn call_with_label_and_no_operand_instructions() {
    let (ast, errors) = parse("CALL foo\nNOP\nNOP\nRET");

    assert!(errors.is_empty());

    let call = match &ast.program[0] {
        Statement::Instruction { instruction, .. } => instruction,
        _ => panic!("expected an instruction"),
    };
    assert!(matches!(call.operand1, Some(Operand::Label(_))));

    for statement in &ast.program[1..] {
        match statement {
            Statement::Instruction { instruction, .. } => {
                assert!(instruction.operand1.is_none());
            }
            _ => panic!("expected an instruction"),
        }
    }
}

#[test]
fn three_operand_instruction() {
    let (ast, errors) = parse("IADD R0, R1, 5");

    assert!(errors.is_empty());
    let instruction = first_instr(&ast);
    assert!(matches!(instruction.operand1, Some(Operand::Register(r)) if r.0 == 0));
    assert!(matches!(instruction.operand2, Some(Operand::Register(r)) if r.0 == 1));
    assert!(matches!(instruction.operand3, Some(Operand::Immediate(5))));
}

#[test]
fn label_before_instruction_across_lines() {
    let (ast, errors) = parse("main:\nMOVE R0, 1\nCALL foo\nRET");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 4);
    assert!(matches!(ast.program[0], Statement::Label { .. }));
    assert!(matches!(ast.program[1], Statement::Instruction { .. }));
    assert!(matches!(ast.program[2], Statement::Instruction { .. }));
    assert!(matches!(ast.program[3], Statement::Instruction { .. }));
}

#[test]
fn align_directive_with_operand() {
    let (ast, errors) = parse(".align 8");

    assert!(errors.is_empty());
    match &ast.program[0] {
        Statement::Directive {
            position,
            directive,
        } => {
            assert!(matches!(directive, Directive::Align { bytes: 8 }));

            assert_eq!(position.start, 0);
            assert_eq!(position.end, 8);
        }
        _ => panic!("expected a directive"),
    }
}

#[test]
fn align_with_the_maximum_value_is_accepted() {
    let (ast, errors) = parse(".align 65536");

    assert!(errors.is_empty());
    assert!(matches!(
        ast.program[0],
        Statement::Directive {
            directive: Directive::Align { bytes: 65536 },
            ..
        }
    ));
}

#[test]
fn align_directive_is_case_insensitive() {
    let (ast, errors) = parse(".ALIGN 4");

    assert!(errors.is_empty());
    assert!(matches!(
        ast.program[0],
        Statement::Directive {
            directive: Directive::Align { bytes: 4 },
            ..
        }
    ));
}

#[test]
fn label_and_directive_on_one_line() {
    let (ast, errors) = parse("main: .align 4");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 2);
    assert!(matches!(ast.program[0], Statement::Label { .. }));
    assert!(matches!(
        ast.program[1],
        Statement::Directive {
            directive: Directive::Align { bytes: 4 },
            ..
        }
    ));
}

#[test]
fn align_directive_after_instruction_line() {
    let (ast, errors) = parse("NOP\n.align 16\nRET");

    assert!(errors.is_empty());
    assert_eq!(ast.program.len(), 3);
    assert!(matches!(ast.program[0], Statement::Instruction { .. }));
    assert!(matches!(
        ast.program[1],
        Statement::Directive {
            directive: Directive::Align { bytes: 16 },
            ..
        }
    ));
    assert!(matches!(ast.program[2], Statement::Instruction { .. }));
}

#[test]
fn minversion_directive_is_case_insensitive() {
    let (ast, errors) = parse(".MINVERSION 0.1.0");

    assert!(errors.is_empty());
    assert!(matches!(
        ast.program[0],
        Statement::Directive {
            directive: Directive::MinVersion {
                major: 0,
                minor: 1,
                patch: 0
            },
            ..
        }
    ));
}
