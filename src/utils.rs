use core::{
    marker::PhantomData,
    mem::size_of,
    ops::Deref,
};
use std::io::{Error, ErrorKind, Read, Write};

use borsh::{BorshDeserialize, BorshSerialize};

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SmallVec<L, T>(Vec<T>, PhantomData<L>);

impl<L, T> SmallVec<L, T> {
    pub fn from_vec(values: Vec<T>) -> Self {
        Self(values, PhantomData)
    }

    pub fn len(&self) -> usize {
        self.0.len()
    }

    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn as_slice(&self) -> &[T] {
        self.0.as_slice()
    }
}

impl<L, T> From<SmallVec<L, T>> for Vec<T> {
    fn from(val: SmallVec<L, T>) -> Self {
        val.0
    }
}

impl<L, T> From<Vec<T>> for SmallVec<L, T> {
    fn from(val: Vec<T>) -> Self {
        Self(val, PhantomData)
    }
}

impl<L, T> Deref for SmallVec<L, T> {
    type Target = Vec<T>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<L, T> BorshSerialize for SmallVec<L, T>
where
    L: BorshSerialize + TryFrom<usize>,
    T: BorshSerialize,
{
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let len = L::try_from(self.0.len()).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                "small vec length does not fit serialized length type",
            )
        })?;

        len.serialize(writer)?;

        for item in self.0.iter() {
            item.serialize(writer)?;
        }

        Ok(())
    }
}

impl<L, T> BorshDeserialize for SmallVec<L, T>
where
    L: BorshDeserialize + TryInto<usize>,
    T: BorshDeserialize,
{
    fn deserialize_reader<R: Read>(reader: &mut R) -> std::io::Result<Self> {
        let serialized_len = L::deserialize_reader(reader)?;
        let len = serialized_len.try_into().map_err(|_| {
            Error::new(
                ErrorKind::InvalidData,
                "small vec length does not fit platform usize",
            )
        })?;

        let mut inner = Vec::with_capacity(len);
        for _ in 0..len {
            inner.push(T::deserialize_reader(reader)?);
        }

        Ok(Self(inner, PhantomData))
    }
}

const _: () = {
    assert!(size_of::<u8>() <= size_of::<usize>());
};
