use borsh::{BorshDeserialize, BorshSerialize};
use p_never_nonce::ensure_never_nonce;
use pinocchio::{
    AccountView, ProgramResult,
    cpi::{Seed, Signer, invoke_signed_with_slice},
    error::ProgramError,
    instruction::{InstructionAccount, InstructionView},
};

use crate::{
    ID,
    errors::NonceGuardError,
    state::{GUARDED_ACCOUNT_SEED, derive_guarded_account_address},
    utils::SmallVec,
};

const MAX_EXECUTE_GUARDED_INSTRUCTIONS: usize = 64;

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct CompiledInstruction {
    pub program_id_index: u8,
    pub account_indices: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct ExecuteGuardedInstructionArgs {
    pub instructions: SmallVec<u8, CompiledInstruction, MAX_EXECUTE_GUARDED_INSTRUCTIONS>,
}

struct ExecuteGuardedAccounts<'a> {
    owner: &'a AccountView,
    guarded_account: &'a AccountView,
    remaining_accounts: &'a [AccountView],
}

struct ExecuteGuardedContext<'a> {
    accounts: ExecuteGuardedAccounts<'a>,
    args: ExecuteGuardedInstructionArgs,
    bump: u8,
}

pub fn process_execute_guarded(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    // Build a fully validated view of the instruction up front so execution can
    // focus only on CPI construction.
    let context = ExecuteGuardedContext::load(accounts, data)?;
    execute_guarded(&context)
}

impl<'a> ExecuteGuardedContext<'a> {
    fn load(accounts: &'a [AccountView], data: &[u8]) -> Result<Self, ProgramError> {
        // Expected account order:
        //   0. owner signer
        //   1. guarded PDA signer
        //   2. instructions sysvar
        //   3.. remaining CPI accounts
        let [owner, guarded_account, instructions_sysvar, remaining_accounts @ ..] = accounts else {
            return Err(ProgramError::NotEnoughAccountKeys);
        };

        if !owner.is_signer() {
            return Err(NonceGuardError::OwnerMustSign.into());
        }

        ensure_never_nonce(instructions_sysvar, NonceGuardError::DurableNonceBlocked)?;

        let args = ExecuteGuardedInstructionArgs::try_from_slice(data)
            .map_err(|_| NonceGuardError::InvalidInstructionData)?;

        let (expected_guarded_account, bump) = derive_guarded_account_address(owner.address());
        if guarded_account.address() != &expected_guarded_account {
            return Err(NonceGuardError::InvalidGuardedPda.into());
        }

        let context = Self {
            accounts: ExecuteGuardedAccounts {
                owner,
                guarded_account,
                remaining_accounts,
            },
            args,
            bump,
        };

        context.validate_instruction_indices()?;

        Ok(context)
    }

    fn validate_instruction_indices(&self) -> Result<(), ProgramError> {
        for instruction in self.args.instructions.iter() {
            // Program and account indices are resolved against the compact
            // execution account space defined by `resolve_account`.
            let program_account = self
                .resolve_account(instruction.program_id_index)
                .map_err(|_| NonceGuardError::ProgramIndexOutOfBounds)?;

            if program_account.address() == &ID {
                return Err(NonceGuardError::SelfCpiBlocked.into());
            }

            for &account_index in instruction.account_indices.iter() {
                self.resolve_account(account_index)
                    .map_err(|_| NonceGuardError::AccountIndexOutOfBounds)?;
            }
        }

        Ok(())
    }

    fn resolve_account(&self, index: u8) -> Result<&'a AccountView, ProgramError> {
        resolve_account(
            index,
            self.accounts.guarded_account,
            self.accounts.remaining_accounts,
        )
    }
}

fn execute_guarded(context: &ExecuteGuardedContext<'_>) -> ProgramResult {
    // The guarded PDA signs every proxied CPI using the same seeds used to
    // derive it from the owner.
    let bump_seed = [context.bump];
    let signer_seeds = [
        Seed::from(GUARDED_ACCOUNT_SEED),
        Seed::from(context.accounts.owner.address().as_ref()),
        Seed::from(bump_seed.as_slice()),
    ];
    let signers = [Signer::from(&signer_seeds)];

    for instruction in context.args.instructions.iter() {
        let program_account = context
            .resolve_account(instruction.program_id_index)
            .map_err(|_| NonceGuardError::ProgramIndexOutOfBounds)?;
        let mut instruction_accounts = Vec::with_capacity(instruction.account_indices.len());
        let mut account_views = Vec::with_capacity(instruction.account_indices.len());

        for &account_index in instruction.account_indices.iter() {
            let account = context
                .resolve_account(account_index)
                .map_err(|_| NonceGuardError::AccountIndexOutOfBounds)?;

            instruction_accounts.push(build_instruction_account(account, account_index == 0));
            account_views.push(account);
        }

        let instruction_view = InstructionView {
            program_id: program_account.address(),
            data: instruction.data.as_slice(),
            accounts: instruction_accounts.as_slice(),
        };

        invoke_signed_with_slice(&instruction_view, account_views.as_slice(), &signers)?;
    }

    Ok(())
}

fn build_instruction_account(account: &AccountView, force_signer: bool) -> InstructionAccount<'_> {
    InstructionAccount::new(
        account.address(),
        account.is_writable(),
        force_signer || account.is_signer(),
    )
}

fn resolve_account<'a>(
    index: u8,
    guarded_account: &'a AccountView,
    remaining_accounts: &'a [AccountView],
) -> Result<&'a AccountView, ProgramError> {
    // Index 0 is reserved for the guarded PDA so callers do not need to pass it
    // again in the tail account list.
    if index == 0 {
        return Ok(guarded_account);
    }

    remaining_accounts
        .get(index as usize - 1)
        .ok_or(ProgramError::NotEnoughAccountKeys)
}
