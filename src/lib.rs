use solana_address::{Address, address};

mod entrypoint;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;
mod allocator;

pub const ID: Address = address!("guardgkc38afzaHQ7LNAtWHKgdUAAwyayhNdY8MPNnw");
