mod country;
mod date;
pub mod query;
mod data;

pub use country::Country;
pub use query::Query;
use date::Date;
pub use date::DateExt;

/// Represents a holiday.
#[derive(Debug, Clone, Copy)]
pub struct Holiday {
    /// two-letter country code defined in ISO 3166-1 alpha-2.
    pub code: Country,
    /// Date of holiday.
    pub date: Date,
    /// Name of holiday.
    pub name: &'static str,
}

pub fn get(country: Country, date: impl Into<Date>) -> query::Iter {
    Query::country(country).and(Query::date(date)).run()
}
pub fn get_in_many<C>(countries: C, date: impl Into<Date>) -> query::Iter where C: IntoIterator<Item = Country> {
    Query::countries(countries).and(Query::date(date)).run()
}
#[inline]
pub fn contains(country: Country, date: impl Into<Date>) -> bool {
    get(country, date).next().is_some()
}
#[inline]
pub fn contains_in_many<C>(countries: C, date: impl Into<Date>) -> bool where C: IntoIterator<Item = Country> {
    get_in_many(countries, date).next().is_some()
}
#[inline]
pub fn query(query: Query) -> query::Iter {
    query.run()
}


/// Error states the holiday crate might encounter.
#[derive(thiserror::Error, Debug, PartialEq, Eq)]
pub enum Error {
    /// Holiday is not available for this country.
    #[error("Holiday is not available for this country")]
    CountryNotAvailable,
    /// Holiday is not available for this year.
    #[error("Holiday is not available for this year")]
    YearNotAvailable,
    /// Holiday database is not initialized yet.
    #[error("Holiday database is not initialized yet")]
    Uninitialized,
    /// Conversion to another date format is not supported.
    #[error("Date is too large for conversion")]
    DateTooLarge,
}

