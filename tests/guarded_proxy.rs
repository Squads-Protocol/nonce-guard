use std::path::PathBuf;

use borsh::to_vec;
use external_signature_program::{
    errors::NonceGuardError,
    instructions::{CompiledInstruction, ExecuteGuardedInstructionArgs},
    utils::SmallVec,
};
use litesvm::LiteSVM;
use solana_account::{Account, ReadableAccount, state_traits::StateMut};
use solana_instruction::{AccountMeta, Instruction, error::InstructionError};
use solana_keypair::Keypair;
use solana_message::Message;
use solana_nonce::{
    state::{Data as NonceData, State as NonceState},
    versions::Versions,
};
use solana_pubkey::{Pubkey, pubkey};
use solana_signer::Signer;
use solana_system_interface::instruction::{advance_nonce_account, create_nonce_account};
use solana_transaction::Transaction;
use solana_transaction_error::TransactionError;

const PROGRAM_ID: Pubkey = pubkey!("ExtSgUPtP3JyKUysFw2S5fpL5fWfUPzGUQLd2bTwftXN");
const GUARDED_ACCOUNT_SEED: &[u8] = b"guarded_proxy";
const TRANSFER_LAMPORTS: u64 = 250_000;

#[test]
fn executes_system_transfer_without_durable_nonce() {
    let mut svm = LiteSVM::new();
    let program_path = program_path();
    svm.add_program_from_file(PROGRAM_ID, &program_path)
        .unwrap_or_else(|err| panic!("failed to load program from {}: {err}", program_path.display()));

    let owner = Keypair::new();
    let owner_pubkey = owner.pubkey();
    let recipient = Pubkey::new_unique();
    let guarded_pda = guarded_pda(&owner_pubkey);

    svm.airdrop(&owner_pubkey, 1_000_000_000).unwrap();
    svm.set_account(
        guarded_pda,
        Account {
            lamports: TRANSFER_LAMPORTS,
            data: vec![],
            owner: solana_sdk_ids::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let instruction = guarded_transfer_instruction(owner_pubkey, guarded_pda, recipient);
    let tx = Transaction::new(
        &[&owner],
        Message::new(&[instruction], Some(&owner_pubkey)),
        svm.latest_blockhash(),
    );

    svm.send_transaction(tx).unwrap();

    let guarded_account = svm.get_account(&guarded_pda).unwrap();
    let recipient_account = svm.get_account(&recipient).unwrap();

    assert_eq!(guarded_account.lamports, 0);
    assert_eq!(recipient_account.lamports, TRANSFER_LAMPORTS);
}

#[test]
fn rejects_system_transfer_when_transaction_uses_durable_nonce() {
    let mut svm = LiteSVM::new();
    let program_path = program_path();
    svm.add_program_from_file(PROGRAM_ID, &program_path)
        .unwrap_or_else(|err| panic!("failed to load program from {}: {err}", program_path.display()));

    let owner = Keypair::new();
    let owner_pubkey = owner.pubkey();
    let recipient = Pubkey::new_unique();
    let nonce = Keypair::new();
    let guarded_pda = guarded_pda(&owner_pubkey);

    svm.airdrop(&owner_pubkey, 1_000_000_000).unwrap();
    svm.set_account(
        guarded_pda,
        Account {
            lamports: TRANSFER_LAMPORTS,
            data: vec![],
            owner: solana_sdk_ids::system_program::id(),
            executable: false,
            rent_epoch: 0,
        },
    )
    .unwrap();

    let create_nonce_ixs = create_nonce_account(&owner_pubkey, &nonce.pubkey(), &owner_pubkey, 1_500_000);
    let create_nonce_tx = Transaction::new(
        &[&owner, &nonce],
        Message::new_with_blockhash(&create_nonce_ixs, Some(&owner_pubkey), &svm.latest_blockhash()),
        svm.latest_blockhash(),
    );
    svm.send_transaction(create_nonce_tx).unwrap();

    let nonce_account = svm.get_account(&nonce.pubkey()).unwrap();
    let nonce_blockhash = nonce_data_from_account(&nonce_account).blockhash();
    let guarded_ix = guarded_transfer_instruction(owner_pubkey, guarded_pda, recipient);
    let advance_nonce_ix = advance_nonce_account(&nonce.pubkey(), &owner_pubkey);
    let message =
        Message::new_with_blockhash(&[advance_nonce_ix, guarded_ix], Some(&owner_pubkey), &nonce_blockhash);
    let tx = Transaction::new(&[&owner], message, nonce_blockhash);

    svm.expire_blockhash();
    let err = svm.send_transaction(tx).unwrap_err();

    assert_eq!(
        err.err,
        TransactionError::InstructionError(1, InstructionError::Custom(NonceGuardError::DurableNonceBlocked as u32))
    );
}

fn guarded_transfer_instruction(owner: Pubkey, guarded_pda: Pubkey, recipient: Pubkey) -> Instruction {
    let execute_args = ExecuteGuardedInstructionArgs {
        instructions: SmallVec::try_from_vec(vec![CompiledInstruction {
            program_id_index: 2,
            account_indices: vec![0, 1],
            data: solana_system_interface::instruction::transfer(&guarded_pda, &recipient, TRANSFER_LAMPORTS)
                .data,
        }])
        .unwrap(),
    };

    Instruction {
        program_id: PROGRAM_ID,
        accounts: vec![
            AccountMeta::new_readonly(owner, true),
            AccountMeta::new(guarded_pda, false),
            AccountMeta::new_readonly(solana_sdk_ids::sysvar::instructions::id(), false),
            AccountMeta::new(recipient, false),
            AccountMeta::new_readonly(solana_sdk_ids::system_program::id(), false),
        ],
        data: to_vec(&execute_args).unwrap(),
    }
}

fn guarded_pda(owner: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[GUARDED_ACCOUNT_SEED, owner.as_ref()], &PROGRAM_ID).0
}

fn program_path() -> PathBuf {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("target")
        .join("deploy")
        .join("external_signature_program.so");

    assert!(
        path.exists(),
        "missing SBF artifact at {}. Run `cargo build-sbf` before executing these litesvm tests.",
        path.display()
    );

    path
}

fn nonce_state_from_account<T: ReadableAccount + StateMut<Versions>>(account: &T) -> NonceState {
    let versions = StateMut::<Versions>::state(account).unwrap();
    NonceState::from(versions)
}

fn nonce_data_from_account<T: ReadableAccount + StateMut<Versions>>(account: &T) -> NonceData {
    match nonce_state_from_account(account) {
        NonceState::Initialized(data) => data,
        NonceState::Uninitialized => panic!("expected initialized nonce account"),
    }
}
