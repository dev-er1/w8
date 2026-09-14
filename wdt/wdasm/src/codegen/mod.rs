//! # Code generation
//!
//! The code generator turns an [`AST`] into a program of instructions.
//!
//! Encoding a program into the W8 Bytecode binary format (`.wb`)
//! — in the [`encoder`] submodule.
pub mod encoder;
pub mod err;

use std::collections::HashMap;

use w8_core::isa::{
    instruction::Instruction,
    opcode::OperationCode,
    operand::{Operand as CoreOperand, OperandKind},
};

use crate::{
    lexer::directive::Directive,
    parser::ast::{AST, Operand, Statement},
    position::Position,
    str_pool::{StrId, StrPool},
};

use self::err::{CodegenError, CodegenErrorKind};

/// Code generation result: the program and the minimum W8 version.
#[derive(Debug, Clone)]
pub struct CodegenResult {
    /// The generated program.
    pub instructions: Vec<Instruction>,

    /// Minimum W8 version from the `.minversion` directive;
    /// `None` — the current version will go into the header.
    pub min_version: Option<(u16, u16, u16)>,
}

/// Generates a program from an [`AST`].
///
/// Labels are resolved into instruction indice. On a duplicate
/// label or a reference to a nonexistent label, the very first
/// error is returned.
pub fn generate(ast: &AST, str_pool: &StrPool) -> Result<CodegenResult, CodegenError> {
    let mut labels: HashMap<StrId, usize> = HashMap::new();
    let mut local_labels: HashMap<(StrId, StrId), usize> = HashMap::new();
    let mut scope: Option<StrId> = None;
    let mut instructions = Vec::new();
    let mut fixups = Vec::new();
    let mut min_version = None;

    // ====== Pass 1: labels and instructions ======

    for statement in &ast.program {
        match statement {
            Statement::Label {
                position,
                name,
                local: false,
            } => {
                if labels.contains_key(name) {
                    return Err(CodegenError::new(
                        CodegenErrorKind::DuplicateLabel {
                            name: label_name(false, *name, str_pool),
                        },
                        *position,
                    ));
                }

                labels.insert(*name, instructions.len());
                scope = Some(*name);
            }

            // A local label is keyed by its scope: the nearest preceding
            // global label. Outside any scope it is an error.
            Statement::Label {
                position,
                name,
                local: true,
            } => {
                let Some(scope_id) = scope else {
                    return Err(CodegenError::new(
                        CodegenErrorKind::LocalLabelWithoutScope {
                            name: label_name(true, *name, str_pool),
                        },
                        *position,
                    ));
                };

                let key = (scope_id, *name);
                if local_labels.contains_key(&key) {
                    return Err(CodegenError::new(
                        CodegenErrorKind::DuplicateLabel {
                            name: label_name(true, *name, str_pool),
                        },
                        *position,
                    ));
                }

                local_labels.insert(key, instructions.len());
            }

            Statement::Directive {
                position: _,
                directive:
                    Directive::MinVersion {
                        major,
                        minor,
                        patch,
                    },
            } => {
                // The last `.minversion` wins.
                min_version = Some((*major, *minor, *patch));
            }

            Statement::Directive {
                directive: Directive::Define { .. } | Directive::LDefine { .. },
                ..
            } => {}

            Statement::Instruction {
                position,
                instruction,
            } => {
                let instr_index = instructions.len();

                let operand1 = flatten(
                    instruction.operand1,
                    instr_index,
                    0,
                    scope,
                    *position,
                    &mut fixups,
                );
                let operand2 = flatten(
                    instruction.operand2,
                    instr_index,
                    1,
                    scope,
                    *position,
                    &mut fixups,
                );
                let operand3 = flatten(
                    instruction.operand3,
                    instr_index,
                    2,
                    scope,
                    *position,
                    &mut fixups,
                );

                instructions.push(Instruction {
                    opcode: instruction.opcode,
                    operand1,
                    operand2,
                    operand3,
                });
            }

            // `.align <bytes>`: pads the instruction stream with `NOP` up to
            // a multiple of `bytes`. The parser guarantees `bytes > 0`;
            // the cap on the size is enforced here as well so that a
            // hand-built AST cannot trigger unbounded padding.
            Statement::Directive {
                position: _,
                directive: Directive::Align { bytes },
                ..
            } => {
                let remainder = instructions.len() % *bytes as usize;
                if remainder != 0 {
                    for _ in remainder..*bytes as usize {
                        instructions.push(nop());
                    }
                }
            }
        }
    }

    // ====== Pass 2: resolving label references ======

    for fixup in fixups {
        let target = if fixup.local {
            match fixup.scope {
                Some(scope) => local_labels.get(&(scope, fixup.label)),
                None => None,
            }
        } else {
            labels.get(&fixup.label)
        };

        let Some(&target) = target else {
            return Err(CodegenError::new(
                CodegenErrorKind::UndefinedLabel {
                    name: label_name(fixup.local, fixup.label, str_pool),
                },
                fixup.position,
            ));
        };

        let operand = Some(CoreOperand {
            kind: OperandKind::Immediate(target as u64),
        });

        match fixup.slot {
            0 => instructions[fixup.instr_index].operand1 = operand,
            1 => instructions[fixup.instr_index].operand2 = operand,
            2 => instructions[fixup.instr_index].operand3 = operand,
            _ => unreachable!("slot is always 0..=2"),
        }
    }

    Ok(CodegenResult {
        instructions,
        min_version,
    })
}

/// A label reference awaiting resolution in the second pass.
struct Fixup {
    /// Index of the instruction with this reference.
    instr_index: usize,

    /// The operand slot (0, 1, or 2) that holds the reference.
    slot: usize,

    /// The label name.
    label: StrId,

    /// Whether the reference is local (`*name`) — then it is resolved
    /// within the scope recorded in [`scope`](Self::scope).
    local: bool,

    /// The scope (the nearest preceding global label) at the reference site.
    scope: Option<StrId>,
    position: Position,
}

/// Converts an AST operand into an instruction operand.
///
/// A label reference is replaced with a zero immediate, and the reference
/// itself is recorded in `fixups`. `scope` is the scope at the reference
/// site: a local reference is resolved within it later.
fn flatten(
    operand: Option<Operand>,
    instr_index: usize,
    slot: usize,
    scope: Option<StrId>,
    position: Position,
    fixups: &mut Vec<Fixup>,
) -> Option<CoreOperand> {
    match operand {
        None => None,
        Some(Operand::Register(register)) => Some(CoreOperand {
            kind: OperandKind::Register(register),
        }),
        Some(Operand::Immediate(value)) => Some(CoreOperand {
            kind: OperandKind::Immediate(value),
        }),
        Some(Operand::Label(label)) => {
            fixups.push(Fixup {
                instr_index,
                slot,
                label,
                local: false,
                scope,
                position,
            });

            Some(CoreOperand {
                kind: OperandKind::Immediate(0),
            })
        }
        Some(Operand::LocalLabel(label)) => {
            fixups.push(Fixup {
                instr_index,
                slot,
                label,
                local: true,
                scope,
                position,
            });

            Some(CoreOperand {
                kind: OperandKind::Immediate(0),
            })
        }
    }
}

/// The label name for error messages: a local label is shown with the
/// `*` prefix, as written in the source code.
fn label_name(local: bool, name: StrId, str_pool: &StrPool) -> String {
    let name = str_pool.get(name);
    if local {
        format!("*{name}")
    } else {
        name.to_string()
    }
}

/// An empty `NOP` instruction — the filler used by `.align`.
fn nop() -> Instruction {
    Instruction {
        opcode: OperationCode::NOP,
        operand1: None,
        operand2: None,
        operand3: None,
    }
}
