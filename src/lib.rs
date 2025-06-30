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
//! ```rust
//! use holidays::Country;
//!
//! let holidays: Vec<_> = holidays::get(
//!     [Country::US, Country::GB], // multiple countries
//!     2025..=2026,                // year range
//! ).collect();
//!
//! for holiday in holidays {
//!     let time: std::time::SystemTime = holiday.date().unwrap();
//!     println!("{} on {}", holiday.name, holiday.date::<SystemTime>().unwrap());
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
pub use query::Iter;
pub use query::selection::Any;

/// Represents a holiday with an associated country, date, and name.
#[derive(Debug, Clone, Copy)]
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
        D: TryFrom<Date, Error = DateConversionError>,
    {
        <D as TryFrom<Date>>::try_from(self.date)
    }
}

/// Queries holidays by countries and date selection.
///
/// # Parameters
/// - `countries`: A value that represents a country selection. It can be:
///   - [`None`] to query all countries,
///   - a single [`Country`], or
///   - any [iterable] container of [`Country`]s (an array, slice, [`Vec`],
///     etc.).
/// - `date`: A value that represents a date range. It can be:
///   - [`None`] to query all available dates,
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
/// # Returns
///
/// An iterator over matching holiday records.
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

/// Error types returned from the crate.
pub mod error {
    pub use crate::country::CountryParseError;
    pub use crate::date::DateConversionError;
}
