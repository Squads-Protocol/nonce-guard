use pinocchio::{AccountView, Address, ProgramResult, entrypoint};

use crate::instructions::process_execute_guarded;

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Address,
    accounts: &[AccountView],
    instruction_data: &[u8],
) -> ProgramResult {
    process_execute_guarded(accounts, instruction_data)
}
