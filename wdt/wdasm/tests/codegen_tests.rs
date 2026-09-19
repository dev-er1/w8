pub mod codegen_tests {
    mod encoder_tests;
    mod error_tests;
    mod generate_tests;

    use w8_core::isa::instruction::Instruction;
    use w8_core::isa::operand::{Operand, OperandKind};
    use w8_core::isa::register::Register;
    use wdasm::codegen;
    use wdasm::codegen::err::CodegenError;
    use wdasm::lexer::Lexer;
    use wdasm::parser::Parser;
    use wdasm::src::SourceCode;
    use wdasm::str_pool::StrPool;

    // Full pipeline: lexer + parser + code generator.
    pub fn codegen(src: &str) -> Result<Vec<Instruction>, CodegenError> {
        codegen_result(src).map(|result| result.instructions)
    }

    // The full pipeline, keeping the whole code generator result
    // (including the declared minimum version).
    pub fn codegen_result(src: &str) -> Result<codegen::CodegenResult, CodegenError> {
        let source = SourceCode::new(src.to_string());
        let mut str_pool = StrPool::from_source(&source);
        let mut lexer = Lexer::new(source, &mut str_pool);
        let tokens = lexer.tokenize().to_vec();

        let mut parser = Parser::new(tokens, &str_pool, &[]);
        let ast = parser.parse().clone();

        codegen::generate(&ast, &str_pool)
    }

    // Register operand.
    pub fn reg(n: u8) -> Operand {
        Operand {
            kind: OperandKind::Register(Register(n)),
        }
    }

    // Immediate operand.
    pub fn imm(value: u64) -> Operand {
        Operand {
            kind: OperandKind::Immediate(value),
        }
    }

    // Compares operands (the types do not implement PartialEq).
    fn same_operand(actual: &Operand, expected: &Operand) -> bool {
        match (actual.kind, expected.kind) {
            (OperandKind::Register(ar), OperandKind::Register(er)) => ar.0 == er.0,
            (OperandKind::Immediate(ai), OperandKind::Immediate(ei)) => ai == ei,
            _ => false,
        }
    }

    // Checks that the actual operand is equal to the expected one.
    pub fn assert_operand_eq(actual: Option<Operand>, expected: Operand) {
        match actual {
            Some(actual) => assert!(
                same_operand(&actual, &expected),
                "expected {expected:?}, got {actual:?}"
            ),
            None => panic!("expected {expected:?}, got None"),
        }
    }

    // Checks that the operand is absent.
    pub fn assert_operand_none(actual: Option<Operand>) {
        assert!(actual.is_none(), "expected no operand, got {actual:?}");
    }
}
