use solana_address::Address;

use crate::ID;

pub const GUARDED_ACCOUNT_SEED: &[u8] = b"guarded_proxy";

pub fn derive_guarded_account_address(owner: &Address) -> (Address, u8) {
    Address::find_program_address(&[GUARDED_ACCOUNT_SEED, owner.as_ref()], &ID)
}
