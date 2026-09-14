use w8_core::vm::ExecuteVariant;
// Helper functions for tests.

use w8_core::{
    isa::{
        instruction::Instruction,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::{VMCallDecision, WVM},
};

// Creating a new register.
pub fn reg(r: u8) -> Operand {
    Operand {
        kind: OperandKind::Register(Register(r)),
    }
}

// Creating an immediate value.
pub fn imm(v: u64) -> Operand {
    Operand {
        kind: OperandKind::Immediate(v),
    }
}

// Run the program on a new VM and return the VM instance.
pub fn run(program: Vec<Instruction>) -> WVM {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.program = program;
    vm.interpretate().expect("execution failed");
    vm
}

// Run the program on an already prepared VM.
pub fn run_on(mut vm: WVM, program: Vec<Instruction>) -> WVM {
    vm.program = program;
    vm.interpretate().expect("execution failed");
    vm
}

// Run the program on an already prepared VM with a `VMCALL` dispatcher.
pub fn run_on_with_dispatch<F>(mut vm: WVM, program: Vec<Instruction>, dispatch: F) -> WVM
where
    F: FnMut(&mut WVM, u64, u64, u64) -> VMCallDecision + 'static,
{
    vm.program = program;
    vm.interpretate_with(dispatch).expect("execution failed");
    vm
}

// Run the program on a new VM and return the execution result.
pub fn interpretate_with_result(
    program: Vec<Instruction>,
) -> Result<WVM, w8_core::vm::err::VMError> {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.program = program;
    vm.interpretate()?;
    Ok(vm)
}
