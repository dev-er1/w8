use w8_core::{
    isa::{
        instruction::Instruction,
        operand::{Operand, OperandKind},
        register::Register,
    },
    vm::{ExecuteVariant, VMCallDecision, WVM},
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

// Size of the JIT code region for the differential runs.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
const JIT_MEMORY_SIZE: usize = 64 * 1024;

// Run the program on a new VM and return the VM instance.
pub fn run(program: Vec<Instruction>) -> WVM {
    run_on(WVM::new(0, ExecuteVariant::default()), program)
}

// Run the program on an already prepared VM.
pub fn run_on(mut vm: WVM, program: Vec<Instruction>) -> WVM {
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    let mut jit = jit_shadow(&vm, program.clone());
    vm.program = program;
    vm.interpretate().expect("execution failed");
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        jit.jit_compile(JIT_MEMORY_SIZE)
            .expect("JIT execution failed");
        assert_same_state(&vm, &jit);
    }
    vm
}

// Run the program on an already prepared VM with a `VMCALL` dispatcher.
pub fn run_on_with_dispatch<F>(mut vm: WVM, program: Vec<Instruction>, dispatch: F) -> WVM
where
    F: FnMut(&mut WVM, u64, u64, u64) -> VMCallDecision + Clone + 'static,
{
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    let mut jit = jit_shadow(&vm, program.clone());
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    let dispatch_for_jit = dispatch.clone();
    vm.program = program;
    vm.interpretate_with(dispatch).expect("execution failed");
    #[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
    {
        jit.jit_compile_with(JIT_MEMORY_SIZE, dispatch_for_jit)
            .expect("JIT execution failed");
        assert_same_state(&vm, &jit);
    }
    vm
}

// Run the program on a new VM and return the execution result.
//
// Stays interpreter-only: it is used by tests for situations the JIT
// cannot faithfully reproduce (out-of-bounds access, division by zero,
// invalid registers).
pub fn interpretate_with_result(
    program: Vec<Instruction>,
) -> Result<WVM, w8_core::vm::err::VMError> {
    let mut vm = WVM::new(0, ExecuteVariant::default());
    vm.program = program;
    vm.interpretate()?;
    Ok(vm)
}

// Builds a fresh JIT VM carrying over the initial state (memory,
// registers, call stack) of the interpreter VM.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
fn jit_shadow(source: &WVM, program: Vec<Instruction>) -> WVM {
    let mut jit = WVM::new(source.memory.len(), ExecuteVariant::ByJIT);
    jit.program = program;
    jit.call_stack = source.call_stack.clone();
    jit.memory = source.memory.clone();
    for i in 0..=Register::MAX_INDEX {
        jit.registers[Register(i)] = source.registers[Register(i)];
    }
    jit
}

// Asserts that the JIT run produced the same observable state as the
// interpreter run: registers, memory, call stack and exit code.
#[cfg(any(target_arch = "x86_64", target_arch = "x86"))]
fn assert_same_state(interpreter: &WVM, jit: &WVM) {
    for i in 0..=Register::MAX_INDEX {
        assert_eq!(
            interpreter.registers[Register(i)],
            jit.registers[Register(i)],
            "JIT or interpreter mismatch in register R{i}",
        );
    }
    assert_eq!(
        interpreter.memory.as_slice(),
        jit.memory.as_slice(),
        "JIT or interpreter mismatch in memory",
    );
    assert_eq!(
        interpreter.call_stack, jit.call_stack,
        "JIT or interpreter mismatch in the call stack",
    );
    assert_eq!(
        interpreter.exit_code, jit.exit_code,
        "JIT or interpreter mismatch in the exit code",
    );
}
