use std::cmp::Reverse;

use crate::{Error, Holiday};

macro_rules! declare_countries {
    ($($code: ident: $str_code: literal $name: literal $val:literal),* $(,)?) => {
        /// Two-letter country codes as specified by ISO 3166-1 alpha-2.
        #[allow(dead_code)]
        #[derive(Debug, Clone, Copy, Eq, PartialEq, Hash)]
        #[repr(u16)]
        pub enum Country {$(
            #[doc = $name]
            $code = $val
        ),*}

        impl Country {
            const CODES: &[&'static str] = &[$(
                $str_code
            ),*];
            const NAMES: &[&'static str] = &[$(
                $name
            ),*];
        }

        impl std::str::FromStr for Country {
            type Err = Error;

            fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
                Ok(match s {
                    $(
                        #[cfg(feature = $str_code)]
                        $str_code => Country::$code,
                    )*
                    _ => return Err(Error::CountryNotAvailable),
                })
            }
        }
    };
}

include!(concat!(env!("OUT_DIR"), "/decl_countries.rs"));

impl Country {
    const COUNT: usize = Self::CODES.len();

    // Returns a long name
    pub fn name(&self) -> &'static str {
        unsafe {
            // SAFETY: Name lookup table is of identical size as country enum
            // value count
            Self::NAMES.get_unchecked(*self as usize)
        }
    }
}

impl std::fmt::Display for Country {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_ref())
    }
}

impl AsRef<str> for Country {
    fn as_ref(&self) -> &str {
        unsafe {
            // SAFETY: Code lookup table is of identical size as country enum
            // value count
            Self::CODES.get_unchecked(*self as usize)
        }
    }
}

const WORD_BITS: usize = 64;
const N_WORDS: usize = Country::COUNT.div_ceil(WORD_BITS);

/// A simple dynamic bitset, storing one bit per country.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CountrySet {
    /// Each `u64` holds 64 bits; we need as many words as it takes
    /// to cover `NUM_COUNTRIES` bits.
    words: [u64; N_WORDS],
}

impl CountrySet {
    /// Create an empty set.
    pub fn new() -> Self {
        CountrySet {
            words: [0; N_WORDS],
        }
    }

    /// Insert one country.
    #[inline]
    pub fn insert(&mut self, country: Country) {
        let idx = country as usize;
        let word = idx / WORD_BITS;
        let bit = idx % WORD_BITS;
        self.words[word] |= 1 << bit;
    }

    /// Check membership.
    #[inline]
    pub fn contains(&self, country: Country) -> bool {
        let idx = country as usize;
        let word = idx / WORD_BITS;
        let bit = idx % WORD_BITS;
        (self.words[word] >> bit) & 1 == 1
    }

    /// Extend from any iterator of countries.
    pub fn extend<I>(&mut self, iter: I)
    where
        I: IntoIterator<Item = Country>,
    {
        for c in iter {
            self.insert(c);
        }
    }

    pub fn iter(&self) -> CountrySetIter {
        CountrySetIter {
            words: self.words,
            word_idx: 0,
            bit_idx: 0,
        }
    }

    pub fn holidays(&self) -> CountrySetHolidayIter {
        let mut iterators: Vec<_> = self
            .iter()
            .map(|i| crate::data::COUNTRY_JUMP_TABLE[i as usize].iter())
            .collect();

        // Seed heap with initial elements
        let mut heap = std::collections::BinaryHeap::with_capacity(iterators.len());
        for (idx, it) in iterators.iter_mut().enumerate() {
            if let Some(&val) = it.next() {
                heap.push(Reverse((val, idx)));
            }
        }

        CountrySetHolidayIter { heap, iterators }
    }
}

impl std::ops::BitOr for CountrySet {
    type Output = Self;

    fn bitor(mut self, rhs: Self) -> Self::Output {
        self |= rhs;
        self
    }
}
impl std::ops::BitOrAssign for CountrySet {
    fn bitor_assign(&mut self, rhs: Self) {
        for w in 0..N_WORDS {
            self.words[w] |= rhs.words[w];
        }
    }
}

impl std::ops::BitAnd for CountrySet {
    type Output = Self;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl std::ops::BitAndAssign for CountrySet {
    fn bitand_assign(&mut self, rhs: Self) {
        for w in 0..N_WORDS {
            self.words[w] &= rhs.words[w];
        }
    }
}


pub(crate) struct CountrySetIter {
    words: [u64; N_WORDS],
    word_idx: usize,
    bit_idx: usize,
}

impl Iterator for CountrySetIter {
    type Item = Country;

    fn next(&mut self) -> Option<Self::Item> {
        while self.word_idx < N_WORDS {
            let word = self.words[self.word_idx];

            if word == 0 {
                self.word_idx += 1;
                self.bit_idx = 0;
                continue;
            }

            while self.bit_idx < 64 {
                let bit = self.bit_idx;
                self.bit_idx += 1;

                if (word >> bit) & 1 == 1 {
                    let idx = self.word_idx * 64 + bit;

                    if idx < Country::COUNT {
                        return Some(unsafe {
                            // SAFETY: bit position (idx) is initially created
                            // by casting Country discriminant into u16
                            std::mem::transmute::<u16, Country>(idx as u16)
                        });
                    } else {
                        return None;
                    }
                }
            }

            self.word_idx += 1;
            self.bit_idx = 0;
        }

        None
    }
}

/// An iterator over merged holiday indices from multiple country jump tables.
pub struct CountrySetHolidayIter {
    // Min‑heap of (value, which iterator)
    heap: std::collections::BinaryHeap<Reverse<(usize, usize)>>,
    // One iterator per country table
    iterators: Vec<std::slice::Iter<'static, usize>>,
}

impl Iterator for CountrySetHolidayIter {
    type Item = &'static Holiday;

    fn next(&mut self) -> Option<Self::Item> {
        // Pop the smallest head element
        let Reverse((val, idx)) = self.heap.pop()?;
        // Replenish that iterator
        if let Some(&next_val) = self.iterators[idx].next() {
            self.heap.push(Reverse((next_val, idx)));
        }
        // Yield the value
        Some(&crate::data::DATA[val])
    }
}
