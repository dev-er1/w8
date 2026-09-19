pub mod parser_tests {
    mod error_tests;
    mod statement_tests;

    use wdasm::lexer::Lexer;
    use wdasm::parser::Parser;
    use wdasm::parser::ast::{AST, Instr, Statement};
    use wdasm::parser::err::ParserError;
    use wdasm::src::SourceCode;
    use wdasm::str_pool::StrPool;

    // Tokenizes and parses the source code, returning the AST and errors.
    pub fn parse(src: &str) -> (AST, Vec<ParserError>) {
        let source = SourceCode::new(src.to_string());
        let mut str_pool = StrPool::from_source(&source);
        let mut lexer = Lexer::new(source, &mut str_pool);
        let tokens = lexer.tokenize().to_vec();

        let mut parser = Parser::new(tokens, &str_pool, &[]);
        let ast = parser.parse().clone();

        (ast, parser.errors)
    }

    // The first instruction of the program.
    pub fn first_instr(ast: &AST) -> &Instr {
        match &ast.program[0] {
            Statement::Instruction { instruction, .. } => instruction,
            _ => panic!("expected the first statement to be an instruction"),
        }
    }
}
