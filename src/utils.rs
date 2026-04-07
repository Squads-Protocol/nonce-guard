use core::{
    fmt,
    marker::PhantomData,
    mem::size_of,
    ops::Deref,
};
use std::io::{Error, ErrorKind, Read, Write};

use borsh::{BorshDeserialize, BorshSerialize};
use smallvec::SmallVec as InlineSmallVec;

pub struct SmallVec<L, T, const N: usize> {
    inner: InlineSmallVec<[T; N]>,
    _len: PhantomData<L>,
}

impl<L, T, const N: usize> SmallVec<L, T, N> {
    pub fn try_from_vec(values: Vec<T>) -> Result<Self, Vec<T>> {
        if values.len() > N {
            return Err(values);
        }

        Ok(Self {
            inner: InlineSmallVec::from_vec(values),
            _len: PhantomData,
        })
    }

    pub fn as_slice(&self) -> &[T] {
        self.inner.as_slice()
    }
}

impl<L, T, const N: usize> Default for SmallVec<L, T, N> {
    fn default() -> Self {
        Self {
            inner: InlineSmallVec::new(),
            _len: PhantomData,
        }
    }
}

impl<L, T, const N: usize> Clone for SmallVec<L, T, N>
where
    T: Clone,
{
    fn clone(&self) -> Self {
        Self {
            inner: self.inner.clone(),
            _len: PhantomData,
        }
    }
}

impl<L, T, const N: usize> fmt::Debug for SmallVec<L, T, N>
where
    T: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.inner.fmt(f)
    }
}

impl<L, T, const N: usize> Deref for SmallVec<L, T, N> {
    type Target = [T];

    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<L, T, const N: usize> BorshSerialize for SmallVec<L, T, N>
where
    L: BorshSerialize + TryFrom<usize>,
    T: BorshSerialize,
{
    fn serialize<W: Write>(&self, writer: &mut W) -> std::io::Result<()> {
        let len = L::try_from(self.inner.len()).map_err(|_| {
            Error::new(
                ErrorKind::InvalidInput,
                "small vec length does not fit serialized length type",
            )
        })?;

        len.serialize(writer)?;

        for item in self.inner.iter() {
            item.serialize(writer)?;
        }

        Ok(())
    }
}

impl<L, T, const N: usize> BorshDeserialize for SmallVec<L, T, N>
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

        if len > N {
            return Err(Error::new(
                ErrorKind::InvalidData,
                "small vec length exceeds inline capacity",
            ));
        }

        let mut inner = InlineSmallVec::<[T; N]>::new();
        for _ in 0..len {
            inner.push(T::deserialize_reader(reader)?);
        }

        Ok(Self {
            inner,
            _len: PhantomData,
        })
    }
}

const _: () = {
    assert!(size_of::<u8>() <= size_of::<usize>());
};
