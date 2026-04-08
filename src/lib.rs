use solana_address::{Address, address};

mod entrypoint;
pub mod errors;
pub mod instructions;
pub mod state;
pub mod utils;
mod allocator;

pub const ID: Address = address!("ExtSgUPtP3JyKUysFw2S5fpL5fWfUPzGUQLd2bTwftXN");
