pub mod loader_tests {
    mod edge_case_tests;
    mod error_tests;
    mod magic_tests;
    mod parse_tests;
    mod version_tests;

    use w8_core::{
        W8_VERSION,
        loader::{W8Loader, err::LoaderError},
    };

    // Creates a minimal valid .wb-file with the current W8 version.
    pub fn make_nb(data: &[u8]) -> Vec<u8> {
        let mut bytes = b"NVMBC".to_vec();
        let mut parts = current_version_parts();
        for _ in 0..3 {
            bytes.extend_from_slice(&parts.next().unwrap_or(0).to_le_bytes());
        }
        bytes.extend_from_slice(data);
        bytes
    }

    // The current W8 version as numeric parts (non-numeric parts
    // are truncated — the same rules as the loader).
    pub fn current_version_parts() -> impl Iterator<Item = u16> {
        W8_VERSION.split('.').map(|part| {
            part.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse()
                .unwrap_or(0)
        })
    }

    // Creates a .wb-file with an arbitrary version.
    pub fn make_nb_with_version(major: u16, minor: u16, patch: u16, data: &[u8]) -> Vec<u8> {
        let mut bytes = b"NVMBC".to_vec();
        bytes.extend_from_slice(&major.to_le_bytes());
        bytes.extend_from_slice(&minor.to_le_bytes());
        bytes.extend_from_slice(&patch.to_le_bytes());
        bytes.extend_from_slice(data);
        bytes
    }

    // Runs the loader and returns the result.
    pub fn run_loader(
        data: Vec<u8>,
    ) -> Result<Vec<w8_core::isa::instruction::Instruction>, LoaderError> {
        W8Loader::new(data).transpile()
    }

    // Creates bytes of the NOP instruction (opcode 0, 0 operands).
    pub fn nop_bytes() -> Vec<u8> {
        vec![0x00, 0x00]
    }

    // Creates bytes of the VMCALL instruction (opcode 52 (0x34), 3 operands).
    pub fn vmcall_bytes(service: u64, address: u64, size: u64) -> Vec<u8> {
        let mut bytes = vec![0x34, 0x03];
        for value in [service, address, size] {
            bytes.push(0x01);
            bytes.extend_from_slice(&value.to_le_bytes());
        }
        bytes
    }

    // Creates bytes of the MOVE instruction with two registers.
    pub fn move_reg_reg(dst: u8, src: u8) -> Vec<u8> {
        vec![0x01, 0x02, 0x00, dst, 0x00, src]
    }

    // Creates bytes of the MOVE instruction with a register and an immediate.
    pub fn move_reg_imm(dst: u8, val: u64) -> Vec<u8> {
        let mut bytes = vec![0x01, 0x02, 0x00, dst, 0x01];
        bytes.extend_from_slice(&val.to_le_bytes());
        bytes
    }
}
