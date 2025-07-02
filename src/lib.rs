//! A fast and flexible holiday query library.
//!
//! This crate provides a static, compile-time generated database of holidays,
//! optimized for flexible and high-performance querying by country and date.
//!
//! Holiday data is sourced from python
//! [`holidays`](https://pypi.org/project/holidays) library.
//!
//! Holiday data is embedded in the binary using [`phf`](https://docs.rs/phf),
//! enabling fast lookups without runtime data loading. Querying is designed to
//! be ergonomic yet powerful, supporting a wide range of inputs such as single
//! dates, ranges, individual countries, and sets of countries.
//!
//! Internally, the query engine dynamically specializes the iteration strategy
//! to the structure of the query. This includes:
//!
//! - Jump tables for precise date lookup
//! - K-way merges for multi-country scanning
//! - Range-based filtering with sorted input
//!
//! While the query layer may perform heap allocations (e.g., when combining
//! iterators or constructing intermediate query plans), it strives to minimize
//! them through static dispatch, specialized iterators, and zero-copy access
//! patterns.
//!
//! # Example
//!
//! ```
//! # use holidays::internal::Date;
//! use holidays::Country;
//!
//! let holidays: Vec<_> = holidays::get_holidays(
//!     [Country::US, Country::GB], // multiple countries
//!     2025..=2026,                // year range
//! ).collect();
//!
//! for holiday in holidays {
//!     let time: std::time::SystemTime = holiday.date().unwrap();
//! #   let time = holiday.date::<Date>().unwrap();
//!     println!("{} on {:?}", holiday.name, time); // pretending time implements Debug
//! }
//! ```
//!
//! # Features
//!
//! - Static, zero-config and zero-allocation holiday database
//! - Efficient date and country queries
//! - Minimal heap allocations, used only when necessary
//! - Extension trait for external types
//! - Optional support for external time libraries: `chrono`, `time`
//!
//! # Performance
//!
//! Although some heap allocations may occur all iteration paths are selected
//! and optimized for performance. Query execution uses tight loops, minimal
//! branching, and completely avoids dynamic dispatch.
//!
//! # When to Use
//!
//! This crate is ideal if you need:
//!
//! - A compile-time embedded holiday dataset
//! - Flexible and expressive querying
//! - High-performance holiday iteration
//!
//! # No Runtime I/O
//!
//! All holiday data is generated at compile time. There is no dependency on
//! runtime files, databases, or network access.

#![warn(missing_docs)]
#![warn(clippy::undocumented_unsafe_blocks)]

mod country;
mod data;
mod date;
mod query;

use date::{Date, DateConversionError};
use query::selection::*;

pub use country::Country;
pub use date::DateExt;
pub use query::selection::Any;
pub use query::Iter;

/// Represents a holiday with an associated country, date, and name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Holiday {
    /// The `Country` this holiday is observed in.
    pub code: Country,
    /// The date of the holiday.
    date: Date,
    /// The name of the holiday.
    pub name: &'static str,
}

impl Holiday {
    /// Returns the date of the holiday in specified format.
    pub fn date<D>(&self) -> Result<D, DateConversionError>
    where
        D: TryFrom<Date>,
    {
        // TryFrom<T> is implemented for all From<T> and error type is
        // Infallible; to unify both conversions error is mapped to
        // DateConversionError. Once specialization is supported, this can be
        // cleaned up.
        <D as TryFrom<Date>>::try_from(self.date).map_err(|_| DateConversionError)
    }
}

/// Queries holidays by countries and date selection and returns an iterator
/// over matching holiday records.
///
/// # Parameters
/// - `countries`: A value that represents a country selection. It can be:
///   - [`Any`] to query all countries,
///   - [`Option`] acts as [`Any`] if `None`,
///   - a single [`Country`], or
///   - any [iterable] container of [`Country`]s (an array, slice, [`Vec`],
///     etc.).
/// - `date`: A value that represents a date range. It can be:
///   - [`Any`] to query all available dates,
///   - [`Option`] acts as [`Any`] if `None`,
///   - a single date, or
///   - a [range] of dates.
///
/// # Type Parameters
/// - `CountryIter`: An [iterable] collection of `Country` values.
/// - `DateLike`: A type that can be converted into a date. Can be any of:
///   - `isize` representing Julian calendar year
///   - [`SystemTime`]
#[cfg_attr(
    feature = "chrono",
    doc = "  - [`chrono::NaiveDate`]\n  - [`chrono::DateTime<Tz>`]"
)]
#[cfg_attr(
    feature = "time",
    doc = "  - [`time::Date`]\n  - [`time::OffsetDateTime`]\n  - [`time::PrimitiveDateTime`]"
)]
/// - `DateRange`: A type implementing [`RangeBounds<DateLike>`], like
///   [`std::ops::Range`] or [`std::ops::RangeInclusive`].
///
/// In most cases these type parameters can be automatically inferred from
/// provided arguments and don't need to be explicitly specified.
/// 
/// # Examples
///
/// To query information about a single country and single date, do dis:
/// ```
/// # use holidays::internal::Date;
/// use holidays::Country;
///
/// let mut holidays = holidays::get_holidays(Country::US, Date::from_ymd(2025, 7, 4));
/// let holiday = holidays.next().expect("missing data");
/// 
/// assert_eq!(holiday.name, "Independence Day");
/// ```
/// 
/// Year ranges can be used to query holidays over a lot o' years:
/// ```
/// use holidays::Country;
///
/// let mut holidays = holidays::get_holidays(Country::JP, 2025..=2026);
/// let observed_holidays = holidays.count();
/// 
/// assert_eq!(observed_holidays, 19);
/// ```
/// 
/// Multiple countries can be queried by providing an iterable in place of a single country:
/// ```
/// # use holidays::internal::Date;
/// use holidays::Country;
///
/// let mut holidays = holidays::get_holidays(&[Country::JP, Country::US], Date::from_ymd(2025, 9, 23));
/// let holiday = holidays.next().unwrap();
/// 
/// assert_eq!(holiday.name, "Autumnal Equinox");
/// // Autumnal Equinox isn't observed in US.
/// assert_eq!(holidays.next(), None);
/// ```
/// 
/// [iterable]: std::iter::IntoIterator
/// [`SystemTime`]: std::time::SystemTime
/// [range]: std::ops::RangeBounds
/// [`RangeBounds<DateLike>`]: std::ops::RangeBounds
pub fn get_holidays<CountryIter, DateLike, DateRange>(
    countries: impl Into<CountrySelection<CountryIter>>,
    date: impl Into<DateSelection<DateLike, DateRange>>,
) -> query::Iter
where
    CountryIter: IntoIterator,
    CountryIter::Item: Into<crate::Country>,
    DateLike: Into<Date> + Clone,
    DateRange: std::ops::RangeBounds<DateLike>,
{
    let country_query = countries.into().into_query();
    let date_query = date.into().into_query();
    country_query.and(date_query).run()
}

/// Returns `true` if any holidays are observed in specified countries and date.
///
/// See [`get_holidays`] function for details on supported arguments.
#[inline]
pub fn is_holiday<CountryIter, DateLike, DateRange>(
    countries: impl Into<CountrySelection<CountryIter>>,
    date: impl Into<DateSelection<DateLike, DateRange>>,
) -> bool
where
    CountryIter: IntoIterator,
    CountryIter::Item: Into<crate::Country>,
    DateLike: Into<Date> + Clone,
    DateRange: std::ops::RangeBounds<DateLike>,
{
    get_holidays(countries, date).next().is_some()
}

/// Returns an iterator that provides dates of first and last event for all
/// given `countries` in requested `DateFormat`.
/// 
/// # Panics
/// 
/// Returned iterator will panic if requested `DateFormat` can't represent date
/// of first or last event for some country.
pub fn get_bounding_entries<DateFormat, CountryIter>(
    countries: impl Into<CountrySelection<CountryIter>>,
) -> impl Iterator<Item = (Country, Option<(DateFormat, DateFormat)>)>
where
    DateFormat: TryFrom<Date, Error = DateConversionError>,
    CountryIter: IntoIterator,
    CountryIter::Item: Into<crate::Country>,
{
    countries.into().bounds().map(|(country, bounds)| {
        (
            country,
            bounds.map(|(min, max)| (min.date().unwrap(), max.date().unwrap())),
        )
    })
}

/// Error types returned from the crate.
pub mod error {
    pub use crate::country::CountryParseError;
    pub use crate::date::DateConversionError;

    macro_rules! error_msg {
        ($err: ty, $message: literal $(, $($arg: tt),+)?) => {
            impl std::fmt::Display for $err {
                fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                    write!(f, $message $(, $(self.$arg),+)?)
                }
            }
            impl core::error::Error for $err {}
        };
    }
    pub(crate) use error_msg;
}

/// This module provides direct access to internals that aren't part of public
/// API and can change at any time without affecting semver.
#[doc(hidden)]
pub mod internal {
    pub use crate::date::Date;
}
