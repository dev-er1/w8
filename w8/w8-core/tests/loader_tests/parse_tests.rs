// Tests on successful parsing of instructions from `.wb`-files.
use w8_core::isa::opcode::OperationCode;

use super::*;

#[test]
fn empty_instruction_stream_returns_empty_vec() {
    let instructions = run_loader(make_nb(&[])).expect("expected successful parse");
    assert!(instructions.is_empty());
}

#[test]
fn single_nop_instruction() {
    let instructions = run_loader(make_nb(&nop_bytes())).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    assert!(matches!(instructions[0].opcode, OperationCode::NOP));
    assert_eq!(instructions[0].operand_count(), 0);
}

#[test]
fn single_vmcall_instruction() {
    let instructions =
        run_loader(make_nb(&vmcall_bytes(0, 0, 1))).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    assert!(matches!(instructions[0].opcode, OperationCode::VMCALL));
    assert_eq!(instructions[0].operand_count(), 3);
}

#[test]
fn move_with_two_registers() {
    let instructions = run_loader(make_nb(&move_reg_reg(0, 1))).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    let instr = &instructions[0];
    assert!(matches!(instr.opcode, OperationCode::MOVE));

    let r0 = instr.operand1.unwrap().expect_register().unwrap();
    let r1 = instr.operand2.unwrap().expect_register().unwrap();
    assert_eq!(r0.0, 0);
    assert_eq!(r1.0, 1);
}

#[test]
fn move_with_register_and_immediate() {
    let instructions =
        run_loader(make_nb(&move_reg_imm(2, 42))).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    let instr = &instructions[0];
    assert!(matches!(instr.opcode, OperationCode::MOVE));

    let dst = instr.operand1.unwrap().expect_register().unwrap();
    let val = instr.operand2.unwrap().expect_immediate().unwrap();
    assert_eq!(dst.0, 2);
    assert_eq!(val, 42);
}

#[test]
fn iadd_with_three_register_operands() {
    let bytes = vec![0x0A, 0x03, 0x00, 0x00, 0x00, 0x01, 0x00, 0x02];

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    let instr = &instructions[0];
    assert!(matches!(instr.opcode, OperationCode::IADD));
    assert_eq!(instr.operand_count(), 3);

    assert_eq!(instr.operand1.unwrap().expect_register().unwrap().0, 0);
    assert_eq!(instr.operand2.unwrap().expect_register().unwrap().0, 1);
    assert_eq!(instr.operand3.unwrap().expect_register().unwrap().0, 2);
}

#[test]
fn iadd_with_mixed_operands() {
    let mut bytes = vec![0x0A, 0x03, 0x00, 0x00, 0x00, 0x01, 0x01];
    bytes.extend_from_slice(&100u64.to_le_bytes());

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    let instr = &instructions[0];
    assert!(matches!(instr.opcode, OperationCode::IADD));

    assert_eq!(instr.operand1.unwrap().expect_register().unwrap().0, 0);
    assert_eq!(instr.operand2.unwrap().expect_register().unwrap().0, 1);
    assert_eq!(instr.operand3.unwrap().expect_immediate().unwrap(), 100);
}

#[test]
fn multiple_instructions_parsed_correctly() {
    let mut bytes = vec![];
    bytes.extend_from_slice(&nop_bytes());
    bytes.extend_from_slice(&vmcall_bytes(0, 0, 1));
    bytes.extend_from_slice(&move_reg_reg(0, 1));

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");

    assert_eq!(instructions.len(), 3);

    assert!(matches!(instructions[0].opcode, OperationCode::NOP));
    assert!(matches!(instructions[1].opcode, OperationCode::VMCALL));
    assert!(matches!(instructions[2].opcode, OperationCode::MOVE));
    assert_eq!(instructions[2].operand_count(), 2);
}

#[test]
fn ret_instruction() {
    let bytes = vec![0x33, 0x00];

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    assert!(matches!(instructions[0].opcode, OperationCode::RET));
}

#[test]
fn jmp_with_immediate_offset() {
    let mut bytes = vec![0x2F, 0x01, 0x01];
    bytes.extend_from_slice(&3u64.to_le_bytes());

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    assert!(matches!(instructions[0].opcode, OperationCode::JMP));
    assert_eq!(
        instructions[0]
            .operand1
            .unwrap()
            .expect_immediate()
            .unwrap(),
        3
    );
}

#[test]
fn jmp_with_register_offset() {
    let bytes = vec![0x2F, 0x01, 0x00, 0x05];

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");

    assert_eq!(instructions.len(), 1);
    assert!(matches!(instructions[0].opcode, OperationCode::JMP));
    assert_eq!(
        instructions[0]
            .operand1
            .unwrap()
            .expect_register()
            .unwrap()
            .0,
        5
    );
}

#[test]
fn instruction_index_is_preserved_in_order() {
    let mut bytes = vec![];
    bytes.extend_from_slice(&nop_bytes());
    bytes.extend_from_slice(&vmcall_bytes(0, 0, 1));
    bytes.extend_from_slice(&nop_bytes());
    bytes.extend_from_slice(&vmcall_bytes(0, 0, 1));
    bytes.extend_from_slice(&nop_bytes());

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");

    assert_eq!(instructions.len(), 5);
    assert!(matches!(instructions[2].opcode, OperationCode::NOP));
    assert!(matches!(instructions[3].opcode, OperationCode::VMCALL));
}

#[test]
fn large_immediate_value() {
    let bytes = move_reg_imm(0, 0xDEAD_BEEF_CAFE_BABE);

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 1);

    let val = instructions[0]
        .operand2
        .unwrap()
        .expect_immediate()
        .unwrap();
    assert_eq!(val, 0xDEAD_BEEF_CAFE_BABE);
}

#[test]
fn instruction_stream_ignores_trailing_data_until_next_opcode() {
    let mut bytes = vec![];
    bytes.extend_from_slice(&move_reg_imm(0, 7));
    bytes.extend_from_slice(&move_reg_imm(1, 8));

    let instructions = run_loader(make_nb(&bytes)).expect("expected successful parse");
    assert_eq!(instructions.len(), 2);

    assert_eq!(
        instructions[0]
            .operand1
            .unwrap()
            .expect_register()
            .unwrap()
            .0,
        0
    );
    assert_eq!(
        instructions[1]
            .operand1
            .unwrap()
            .expect_register()
            .unwrap()
            .0,
        1
    );
}
