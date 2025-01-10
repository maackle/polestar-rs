use std::{
    fmt::{Binary, Debug, Write},
    marker::PhantomData,
    num::Wrapping,
    ops::{Add, BitAnd, Shl, Sub},
};

use exhaustive::*;
use num_traits::*;
use polestar::id::UpTo;

type Loc = u8;

const fn max_grain<T>() -> u32 {
    (size_of::<T>() * 8).ilog2()
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Clone, Copy)]
pub struct Arq<Space> {
    pub grain: u32,
    pub start: Loc,
    pub len: Loc,
    space: PhantomData<Space>,
}

impl<Space> Arq<Space> {
    pub fn new(grain: u32, start: Loc, len: Loc) -> Self {
        assert!(grain <= max_grain::<Space>());
        let divs = 2u8.pow(grain);
        assert!(start < divs);
        assert!(len > 0);
        assert!(len <= divs);
        Self {
            grain,
            start,
            len,
            space: PhantomData,
        }
    }
}

impl<Space> Exhaustive for Arq<Space> {
    fn generate(u: &mut DataSourceTaker) -> Result<Self> {
        // let g = MAX_GRAIN.min(size_of::<Space>() * 8);
        let grain = u.choice(1 + max_grain::<Space>() as usize)? as u32;
        let divs = 2usize.pow(grain);
        let start = u.choice(divs)? as u8;
        let len = 1 + u.choice(divs)? as u8;

        Ok(Self {
            grain,
            start,
            len,
            space: PhantomData,
        })
    }
}

pub trait Spacey:
    Debug
    + Copy
    + Ord
    + Eq
    + Binary
    + BitAnd<Output = Self>
    + From<u8>
    + Zero
    + One
    + WrappingSub
    + Pow<u32, Output = Self>
{
    const BITS: usize;
    fn rotate_left(&self, n: u32) -> Self;
    fn reverse_bits(&self) -> Self;
    fn to_ascii(&self) -> String;
}

macro_rules! impl_spacey_primitive {
    ($t:ty , $n:literal, $fmt:literal) => {
        impl Spacey for $t {
            const BITS: usize = $n;

            fn rotate_left(&self, n: u32) -> Self {
                <$t>::rotate_left(*self, n)
            }

            fn reverse_bits(&self) -> Self {
                <$t>::reverse_bits(*self)
            }

            fn to_ascii(&self) -> String {
                format!($fmt, *self)
            }
        }
    };
}

impl_spacey_primitive!(u8, 8, "{:08b}");
impl_spacey_primitive!(u16, 16, "{:016b}");
impl_spacey_primitive!(u32, 32, "{:032b}");
impl_spacey_primitive!(u64, 64, "{:064b}");
impl_spacey_primitive!(u128, 128, "{:0128b}");

impl<Space> Arq<Space>
where
    Space: Spacey,
{
    pub fn normalize(&self) -> Self {
        todo!()
    }

    pub fn to_space(&self) -> Space {
        let Self {
            grain, start, len, ..
        } = *self;

        let chunk = Space::BITS / 2usize.pow(grain);
        let pow = chunk as u32 * len as u32;
        assert!(pow <= Space::BITS as u32);
        if pow == Space::BITS as u32 {
            return Space::zero().wrapping_sub(&Space::one());
        }
        let mask = Space::from(2);
        let mask = mask.pow(pow);
        // println!("{:08b}", mask);
        let mask = mask.wrapping_sub(&Space::one());
        // println!("{:08b}", mask);
        let mask = mask.rotate_left(start as u32 * chunk as u32);
        // println!("{:08b}", mask);

        mask.reverse_bits()
    }

    pub fn to_ascii(&self) -> String {
        self.to_space().to_ascii()
    }
}

#[cfg(test)]
mod tests {
    use itertools::Itertools;

    use super::*;

    #[test]
    fn arq_exhaustive() {
        let all = Arq::<u32>::iter_exhaustive(None).collect_vec();
        for arq in all.iter() {
            println!("{}", arq.to_ascii());
        }
        let count = all.len();
        println!("count: {count}");
    }

    #[test]
    fn arq_unit_tests() {
        type A = Arq<u8>;

        assert_eq!(max_grain::<u8>(), 3);
        assert_eq!(max_grain::<u16>(), 4);
        assert_eq!(max_grain::<u32>(), 5);
        assert_eq!(max_grain::<u64>(), 6);
        assert_eq!(max_grain::<u128>(), 7);

        // 0

        assert_eq!(A::new(0, 0, 1).to_ascii(), "11111111");
        assert_eq!(A::new(1, 0, 2).to_ascii(), "11111111");
        assert_eq!(A::new(2, 0, 4).to_ascii(), "11111111");
        assert_eq!(A::new(3, 0, 8).to_ascii(), "11111111");

        assert_eq!(A::new(1, 0, 1).to_ascii(), "11110000");
        assert_eq!(A::new(2, 0, 1).to_ascii(), "11000000");
        assert_eq!(A::new(3, 0, 1).to_ascii(), "10000000");

        assert_eq!(A::new(2, 0, 2).to_ascii(), "11110000");
        assert_eq!(A::new(3, 0, 4).to_ascii(), "11110000");
        assert_eq!(A::new(3, 0, 3).to_ascii(), "11100000");
        assert_eq!(A::new(3, 0, 2).to_ascii(), "11000000");
        assert_eq!(A::new(3, 0, 1).to_ascii(), "10000000");

        // 1

        assert_eq!(A::new(1, 1, 2).to_ascii(), "11111111");
        assert_eq!(A::new(2, 1, 4).to_ascii(), "11111111");
        assert_eq!(A::new(3, 1, 8).to_ascii(), "11111111");

        assert_eq!(A::new(1, 1, 1).to_ascii(), "00001111");
        assert_eq!(A::new(2, 1, 1).to_ascii(), "00110000");
        assert_eq!(A::new(3, 1, 1).to_ascii(), "01000000");

        assert_eq!(A::new(2, 1, 2).to_ascii(), "00111100");
        assert_eq!(A::new(3, 1, 4).to_ascii(), "01111000");
        assert_eq!(A::new(3, 1, 3).to_ascii(), "01110000");
        assert_eq!(A::new(3, 1, 2).to_ascii(), "01100000");
        assert_eq!(A::new(3, 1, 1).to_ascii(), "01000000");

        // wrap

        assert_eq!(A::new(3, 0, 4).to_ascii(), "11110000");
        assert_eq!(A::new(3, 1, 4).to_ascii(), "01111000");
        assert_eq!(A::new(3, 2, 4).to_ascii(), "00111100");
        assert_eq!(A::new(3, 3, 4).to_ascii(), "00011110");
        assert_eq!(A::new(3, 4, 4).to_ascii(), "00001111");
        assert_eq!(A::new(3, 5, 4).to_ascii(), "10000111");
        assert_eq!(A::new(3, 6, 4).to_ascii(), "11000011");
        assert_eq!(A::new(3, 7, 4).to_ascii(), "11100001");
    }
}
