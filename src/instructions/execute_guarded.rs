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

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct CompiledInstruction {
    pub program_id_index: u8,
    pub account_indices: Vec<u8>,
    pub data: Vec<u8>,
}

#[derive(BorshDeserialize, BorshSerialize, Clone, Debug)]
pub struct ExecuteGuardedInstructionArgs {
    pub instructions: SmallVec<u8, CompiledInstruction>,
}

struct ExecuteGuardedContext<'a> {
    owner: &'a AccountView,
    guarded_account: &'a AccountView,
    remaining_accounts: &'a [AccountView],
    bump: u8,
}

pub fn process_execute_guarded(accounts: &[AccountView], data: &[u8]) -> ProgramResult {
    let context = ExecuteGuardedContext::load(accounts)?;
    let args = ExecuteGuardedInstructionArgs::try_from_slice(data)
        .map_err(|_| NonceGuardError::InvalidInstructionData)?;

    // The guarded PDA signs every proxied CPI using the same seeds used to
    // derive it from the owner.
    let bump_seed = [context.bump];
    let signer_seeds = [
        Seed::from(GUARDED_ACCOUNT_SEED),
        Seed::from(context.owner.address().as_ref()),
        Seed::from(bump_seed.as_slice()),
    ];
    let signers = [Signer::from(&signer_seeds)];

    for instruction in args.instructions.iter() {
        // Program and account indices are resolved against the compact
        // execution account space defined by `resolve_account`.
        let program_account = resolve_account(
            instruction.program_id_index,
            context.guarded_account,
            context.remaining_accounts,
        )
        .map_err(|_| NonceGuardError::ProgramIndexOutOfBounds)?;

        if program_account.address() == &ID {
            return Err(NonceGuardError::SelfCpiBlocked.into());
        }

        let mut instruction_accounts = Vec::with_capacity(instruction.account_indices.len());
        let mut account_views = Vec::with_capacity(instruction.account_indices.len());

        for &account_index in instruction.account_indices.iter() {
            let account = resolve_account(
                account_index,
                context.guarded_account,
                context.remaining_accounts,
            )
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

impl<'a> ExecuteGuardedContext<'a> {
    fn load(accounts: &'a [AccountView]) -> Result<Self, ProgramError> {
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

        let (expected_guarded_account, bump) = derive_guarded_account_address(owner.address());
        if guarded_account.address() != &expected_guarded_account {
            return Err(NonceGuardError::InvalidGuardedPda.into());
        }

        Ok(Self {
            owner,
            guarded_account,
            remaining_accounts,
            bump,
        })
    }
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
