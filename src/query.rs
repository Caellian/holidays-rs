use crate::country::{Country, CountrySet, CountrySetHolidayIter};
use crate::{date::Date, Holiday};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Query {
    countries: CountrySet,
    date_filter: Option<DateQuery>,
}

impl Query {
    pub fn country(value: Country) -> Self {
        Query {
            countries: {
                let mut countries = CountrySet::new();
                countries.insert(value);
                countries
            },
            date_filter: None,
        }
    }

    pub fn countries<I>(value: I) -> Self
    where
        I: IntoIterator<Item = Country>,
    {
        Query {
            countries: {
                let mut countries = CountrySet::new();
                countries.extend(value);
                countries
            },
            date_filter: None,
        }
    }

    pub fn year(value: isize) -> Self {
        Query {
            countries: CountrySet::new(),
            date_filter: Some(DateQuery::year(value)),
        }
    }

    pub fn year_range<R: std::ops::RangeBounds<isize>>(value: R) -> Self {
        Query {
            countries: CountrySet::new(),
            date_filter: DateQuery::year_range(value),
        }
    }

    pub fn date(value: impl Into<Date>) -> Self {
        Query {
            countries: CountrySet::new(),
            date_filter: Some(DateQuery::date(value)),
        }
    }

    pub fn date_range<D, R>(value: R) -> Self
    where
        D: Into<Date> + Clone,
        R: std::ops::RangeBounds<D>,
    {
        Query {
            countries: CountrySet::new(),
            date_filter: DateQuery::date_range(value),
        }
    }

    pub fn and(mut self, other: Self) -> Self {
        self.countries |= other.countries;
        self.date_filter = match (self.date_filter, other.date_filter) {
            (None, Some(it)) => Some(it),
            (Some(it), None) => Some(it),
            (Some(a), Some(b)) => Some(a & b),
            (None, None) => None,
        };
        self
    }

    pub(crate) fn run(&self) -> Iter {
        match self.date_filter {
            Some(empty) if empty.is_empty() => Iter::Empty,
            Some(DateQuery::Exact(date)) => Iter::Exact(IterExact {
                inner: self.countries.iter(),
                date,
            }),
            Some(date_query) => Iter::DateRange(IterDateRange {
                range: date_query.as_data_range(),
                countries: self.countries,
            }),
            None => Iter::NoDate(self.countries.holidays()),
        }
    }
}

impl std::ops::BitAnd for Query {
    type Output = Self;

    fn bitand(mut self, rhs: Self) -> Self::Output {
        self &= rhs;
        self
    }
}
impl std::ops::BitAndAssign for Query {
    fn bitand_assign(&mut self, rhs: Self) {
        self.countries |= rhs.countries;
        self.date_filter = match (self.date_filter, rhs.date_filter) {
            (None, Some(it)) => Some(it),
            (Some(it), None) => Some(it),
            (Some(a), Some(b)) => Some(a & b),
            (None, None) => None,
        };
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum DateQuery {
    FromDate(Date),
    ToDate(Date),
    Exact(Date),
    DateRange(Date, Date),
}

impl DateQuery {
    const EMPTY: DateQuery = DateQuery::DateRange(Date(0), Date(0));

    #[inline(always)]
    fn year(value: isize) -> Self {
        DateQuery::DateRange(Date::from_year(value), Date::from_year(value + 1))
    }

    #[inline(always)]
    fn year_range<R>(value: R) -> Option<Self>
    where
        R: std::ops::RangeBounds<isize>,
    {
        let start = match value.start_bound() {
            std::ops::Bound::Included(it) => Date::from_year(*it),
            std::ops::Bound::Excluded(it) => Date::from_year(it + 1),
            std::ops::Bound::Unbounded => match value.end_bound() {
                std::ops::Bound::Included(it) => {
                    return Some(DateQuery::ToDate(Date::from_year(it + 1)))
                }
                std::ops::Bound::Excluded(it) => {
                    return Some(DateQuery::ToDate(Date::from_year(*it)))
                }
                std::ops::Bound::Unbounded => return None,
            },
        };

        let end = match value.end_bound() {
            std::ops::Bound::Included(it) => Date::from_year(it + 1),
            std::ops::Bound::Excluded(it) => Date::from_year(*it),
            std::ops::Bound::Unbounded => match value.start_bound() {
                std::ops::Bound::Included(it) => {
                    return Some(DateQuery::FromDate(Date::from_year(*it)))
                }
                std::ops::Bound::Excluded(it) => {
                    return Some(DateQuery::FromDate(Date::from_year(it + 1)))
                }
                std::ops::Bound::Unbounded => return None, // unreachable
            },
        };

        Some(DateQuery::DateRange(start, end))
    }

    #[inline(always)]
    fn date(value: impl Into<Date>) -> Self {
        DateQuery::Exact(value.into())
    }

    #[inline(always)]
    fn date_range<D, R>(value: R) -> Option<Self>
    where
        D: Into<Date> + Clone,
        R: std::ops::RangeBounds<D>,
    {
        let start = match value.start_bound() {
            std::ops::Bound::Included(it) => it.clone().into(),
            std::ops::Bound::Excluded(it) => {
                let mut it = it.clone().into();
                it.0 += 1;
                it
            }
            std::ops::Bound::Unbounded => match value.end_bound() {
                std::ops::Bound::Included(it) => {
                    return Some(DateQuery::ToDate({
                        let mut it = it.clone().into();
                        it.0 += 1;
                        it
                    }))
                }
                std::ops::Bound::Excluded(it) => return Some(DateQuery::ToDate(it.clone().into())),
                std::ops::Bound::Unbounded => return None,
            },
        };

        let end = match value.end_bound() {
            std::ops::Bound::Included(it) => {
                let mut it = it.clone().into();
                it.0 += 1;
                it
            }
            std::ops::Bound::Excluded(it) => it.clone().into(),
            std::ops::Bound::Unbounded => match value.start_bound() {
                std::ops::Bound::Included(it) => {
                    return Some(DateQuery::FromDate(it.clone().into()))
                }
                std::ops::Bound::Excluded(it) => {
                    return Some(DateQuery::FromDate({
                        let mut it = it.clone().into();
                        it.0 += 1;
                        it
                    }))
                }
                std::ops::Bound::Unbounded => return None, // unreachable
            },
        };

        Some(DateQuery::DateRange(start, end))
    }

    fn is_empty(&self) -> bool {
        match self {
            DateQuery::DateRange(a, b) => a >= b,
            _ => false,
        }
    }

    fn as_data_range(&self) -> std::ops::Range<usize> {
        const DATA_LEN: usize = crate::data::DATA.len();
        match self {
            DateQuery::DateRange(from, to) => {
                match (
                    crate::data::date_to_index(*from),
                    crate::data::date_to_index(*to),
                ) {
                    (Some(a), Some(b)) => a..b,
                    (Some(a), None) => a..DATA_LEN,
                    (None, Some(b)) => 0..b,
                    (None, None) => 0..0,
                }
            }
            DateQuery::FromDate(date) => match crate::data::date_to_index(*date) {
                Some(it) => it..DATA_LEN,
                None => DATA_LEN..DATA_LEN,
            },
            DateQuery::ToDate(date) => match crate::data::date_to_index(*date) {
                Some(it) => 0..it,
                None => 0..0,
            },
            DateQuery::Exact(date) => {
                let from = match crate::data::date_to_index(*date) {
                    Some(it) => it,
                    None => return DATA_LEN..DATA_LEN,
                };
                match crate::data::date_to_index(Date(date.0 + 1)) {
                    Some(to) => from..to,
                    None => from..DATA_LEN,
                }
            }
        }
    }
}

impl std::ops::BitAnd for DateQuery {
    type Output = Self;

    fn bitand(self, rhs: Self) -> Self::Output {
        match (self, rhs) {
            (DateQuery::FromDate(a), DateQuery::FromDate(b)) => DateQuery::FromDate(a.max(b)),
            (DateQuery::ToDate(a), DateQuery::ToDate(b)) => DateQuery::ToDate(a.min(b)),
            (DateQuery::Exact(a), DateQuery::Exact(b)) => {
                if a != b {
                    DateQuery::EMPTY
                } else {
                    DateQuery::Exact(a)
                }
            }

            (DateQuery::FromDate(a), DateQuery::Exact(b))
            | (DateQuery::Exact(b), DateQuery::FromDate(a)) => {
                if a > b {
                    DateQuery::EMPTY
                } else {
                    DateQuery::Exact(b)
                }
            }
            (DateQuery::ToDate(a), DateQuery::Exact(b))
            | (DateQuery::Exact(b), DateQuery::ToDate(a)) => {
                if b >= a {
                    DateQuery::EMPTY
                } else {
                    DateQuery::Exact(b)
                }
            }
            (DateQuery::Exact(a), DateQuery::DateRange(b_from, b_to))
            | (DateQuery::DateRange(b_from, b_to), DateQuery::Exact(a)) => {
                if b_from > a || b_to <= a {
                    DateQuery::EMPTY
                } else {
                    DateQuery::Exact(a)
                }
            }
            (DateQuery::FromDate(a), DateQuery::ToDate(b))
            | (DateQuery::ToDate(b), DateQuery::FromDate(a)) => {
                if a >= b {
                    DateQuery::EMPTY
                } else {
                    DateQuery::DateRange(a, b)
                }
            }
            (DateQuery::FromDate(a), DateQuery::DateRange(b_from, b_to))
            | (DateQuery::DateRange(b_from, b_to), DateQuery::FromDate(a)) => {
                let from = a.max(b_from);
                if from >= b_to {
                    DateQuery::EMPTY
                } else {
                    DateQuery::DateRange(from, b_to)
                }
            }
            (DateQuery::ToDate(a), DateQuery::DateRange(b_from, b_to))
            | (DateQuery::DateRange(b_from, b_to), DateQuery::ToDate(a)) => {
                let to = a.max(b_to);
                if to >= b_from {
                    DateQuery::EMPTY
                } else {
                    DateQuery::DateRange(b_from, to)
                }
            }
            (DateQuery::DateRange(a_from, a_to), DateQuery::DateRange(b_from, b_to)) => {
                let from = a_from.max(b_from);
                let to = a_to.min(b_to);
                if to >= from {
                    DateQuery::EMPTY
                } else {
                    DateQuery::DateRange(from, to)
                }
            }
        }
    }
}

mod detail {
    use super::*;
    pub struct IterExact {
        pub(super) inner: crate::country::CountrySetIter,
        pub(super) date: Date,
    }

    pub struct IterDateRange {
        pub(super) range: std::ops::Range<usize>,
        pub(super) countries: CountrySet,
    }
}
use detail::*;

/// Iterator over holiday query results.
pub enum Iter {
    Empty,
    Exact(IterExact),
    DateRange(IterDateRange),
    NoDate(CountrySetHolidayIter),
}

impl Iterator for Iter {
    type Item = &'static Holiday;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Iter::Empty => None,
            Iter::Exact(IterExact { inner, date }) => loop {
                let next = inner.next()?;
                if let Some(it) = crate::data::country_date_to_holiday(next, *date) {
                    return Some(it);
                }
            },
            Iter::DateRange(IterDateRange { range, countries }) => loop {
                let i = range.next()?;
                let result = &crate::data::DATA[i];
                if countries.contains(result.code) {
                    return Some(result);
                }
            },
            Iter::NoDate(inner) => inner.next(),
        }
    }
}
