use pinocchio::error::ProgramError;

#[repr(u32)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NonceGuardError {
    InvalidInstructionData = 0,
    OwnerMustSign = 1,
    InvalidGuardedPda = 2,
    AccountIndexOutOfBounds = 3,
    ProgramIndexOutOfBounds = 4,
    SelfCpiBlocked = 5,
    DurableNonceBlocked = 6,
}

impl From<NonceGuardError> for ProgramError {
    fn from(value: NonceGuardError) -> Self {
        ProgramError::Custom(value as u32)
    }
}
