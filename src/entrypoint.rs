use pinocchio::{AccountView, Address, ProgramResult, program_entrypoint};

use crate::instructions::process_execute_guarded;

// Entrypoint Configuration
#[cfg(target_os = "solana")]
#[global_allocator]
pub static A: crate::allocator::BumpAllocator = crate::allocator::BumpAllocator;
program_entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    process_execute_guarded(accounts, instruction_data)
}
