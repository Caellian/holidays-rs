use std::time::Duration;
use crate::query::selection::CountrySelection;

#[derive(Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[repr(transparent)]
pub struct Date(
    /// Days since 1st of January, 1970. (UNIX epoch)
    pub(crate) isize,
);

impl Date {
    pub const fn from_ymd(year: isize, month: usize, day: usize) -> Self {
        // Source: https://howardhinnant.github.io/date_algorithms.html

        let y = year;
        let m = month as isize;
        let d = day as isize;

        let adjusted_year = y - if m <= 2 { 1 } else { 0 };

        let era = if adjusted_year >= 0 {
            adjusted_year / 400
        } else {
            (adjusted_year - 399) / 400
        };

        let year_of_era = adjusted_year - era * 400;
        let month_part = if m > 2 { m - 3 } else { m + 9 };
        let day_of_year = (153 * month_part + 2) / 5 + d - 1;
        let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;

        let days_since_julian = era * 146097 + day_of_era;

        Self(days_since_julian - 719163)
    }

    #[inline]
    pub const fn from_year(year: isize) -> Self {
        Self::from_ymd(year, 1, 1)
    }

    pub const fn ymd(&self) -> (isize, usize, usize) {
        // Source: https://howardhinnant.github.io/date_algorithms.html

        let julian_day = self.0 + 719163;
        let shifted = julian_day + 32044;

        let era = (4 * shifted + 3) / 146097;
        let day_of_era = shifted - (146097 * era) / 4;
        let year_of_era = (4 * day_of_era + 3) / 1461;
        let day_of_year = day_of_era - (1461 * year_of_era) / 4;
        let month_part = (5 * day_of_year + 2) / 153;

        let day = day_of_year - (153 * month_part + 2) / 5 + 1;
        let month = (month_part + 3 - 1) % 12 + 1;
        let year = 100 * era + year_of_era - 4800 + (month_part + 3) / 12;

        (year, month as usize, day as usize)
    }

    /// Day of the month
    #[inline]
    pub const fn day(&self) -> usize {
        self.ymd().2
    }

    /// Month of the year
    #[inline]
    pub const fn month(&self) -> usize {
        self.ymd().1
    }

    /// Year
    #[inline]
    pub const fn year(&self) -> isize {
        self.ymd().0
    }

    pub const fn days_since(&self, other: &Self) -> Result<usize, usize> {
        if self.0 > other.0 {
            Ok((self.0 - other.0) as usize)
        } else {
            Err((other.0 - self.0) as usize)
        }
    }

    pub const fn duration_since(&self, other: &Self) -> Result<Duration, Duration> {
        if self.0 > other.0 {
            Ok(Duration::from_secs(
                SECONDS_IN_DAY as u64 * (self.0 - other.0) as u64,
            ))
        } else {
            Err(Duration::from_secs(
                SECONDS_IN_DAY as u64 * (other.0 - self.0) as u64,
            ))
        }
    }
}

/// An `isize` value is treated like a year in Julian calendar.
impl From<isize> for Date {
    fn from(value: isize) -> Self {
        Date::from_year(value)
    }
}

const SECONDS_IN_DAY: isize = 86400;

impl TryFrom<Date> for std::time::SystemTime {
    type Error = DateConversionError;

    fn try_from(value: Date) -> Result<Self, Self::Error> {
        if value.0 > u64::MAX as isize / SECONDS_IN_DAY {
            return Err(DateConversionError);
        }
        assert!(
            value.0 <= u64::MAX as isize / SECONDS_IN_DAY,
            "date too large"
        );
        Ok(std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(value.0 as u64 * SECONDS_IN_DAY as u64))
    }
}

impl From<std::time::SystemTime> for Date {
    fn from(value: std::time::SystemTime) -> Self {
        let days = match value.duration_since(std::time::SystemTime::UNIX_EPOCH) {
            Ok(duration) => duration.as_secs() as isize / SECONDS_IN_DAY,
            Err(err) => -(err.duration().as_secs() as isize / SECONDS_IN_DAY),
        };

        Date(days)
    }
}

#[cfg(feature = "chrono")]
impl TryFrom<Date> for chrono::NaiveDate {
    type Error = DateConversionError;

    fn try_from(value: Date) -> Result<Self, Self::Error> {
        if value.0 > i32::MAX as isize - 719163 {
            return Err(DateConversionError);
        }
        chrono::NaiveDate::from_num_days_from_ce_opt(value.0 as i32 + 719163)
            .ok_or(DateConversionError)
    }
}
#[cfg(feature = "chrono")]
impl TryFrom<Date> for chrono::DateTime<chrono::Utc> {
    type Error = DateConversionError;

    fn try_from(value: Date) -> Result<Self, Self::Error> {
        let naive = chrono::NaiveDate::try_from(value)?
            .and_hms_opt(0, 0, 0)
            .ok_or(DateConversionError)?;

        Ok(chrono::TimeZone::from_utc_datetime(&chrono::Utc, &naive))
    }
}
#[cfg(feature = "chrono")]
impl TryFrom<Date> for chrono::DateTime<chrono::Local> {
    type Error = DateConversionError;

    #[inline]
    fn try_from(value: Date) -> Result<Self, Self::Error> {
        let dt_utc = chrono::DateTime::<chrono::Utc>::try_from(value)?;
        Ok(dt_utc.with_timezone(&chrono::Local))
    }
}

#[cfg(feature = "chrono")]
impl From<chrono::NaiveDate> for Date {
    fn from(value: chrono::NaiveDate) -> Self {
        let epoch = chrono::NaiveDate::from_ymd_opt(1970, 1, 1).unwrap();
        let days = value.signed_duration_since(epoch).num_days();
        Date(days as isize)
    }
}
#[cfg(feature = "chrono")]
impl From<chrono::DateTime<chrono::Utc>> for Date {
    #[inline]
    fn from(value: chrono::DateTime<chrono::Utc>) -> Self {
        Date::from(value.date_naive())
    }
}
#[cfg(feature = "chrono")]
impl From<chrono::DateTime<chrono::Local>> for Date {
    #[inline]
    fn from(value: chrono::DateTime<chrono::Local>) -> Self {
        Date::from(value.naive_local().date())
    }
}

#[cfg(feature = "time")]
impl TryFrom<Date> for time::Date {
    type Error = DateConversionError;

    fn try_from(value: Date) -> Result<Self, Self::Error> {
        if value.0.abs() > i64::MAX as isize {
            return Err(DateConversionError);
        }

        // `Date` days since 1970-01-01; time::Date::from_julian_day starts from -4713-11-24
        let epoch = time::Date::from_calendar_date(1970, time::Month::January, 1)
            .map_err(|_| DateConversionError)?;
        Ok(epoch
            .saturating_add(time::Duration::days(value.0 as i64)))
    }
}

#[cfg(feature = "time")]
impl TryFrom<Date> for time::OffsetDateTime {
    type Error = DateConversionError;

    fn try_from(value: Date) -> Result<Self, Self::Error> {
        let date = time::Date::try_from(value)?;
        let date = date.with_hms(0, 0, 0).unwrap();
        Ok(date.assume_utc())
    }
}

#[cfg(feature = "time")]
impl TryFrom<Date> for time::PrimitiveDateTime {
    type Error = DateConversionError;

    fn try_from(value: Date) -> Result<Self, Self::Error> {
        let date = time::Date::try_from(value)?;
        let date = date.with_hms(0, 0, 0).unwrap();
        Ok(date)
    }
}

#[cfg(feature = "time")]
impl From<time::Date> for Date {
    fn from(value: time::Date) -> Self {
        let epoch = time::Date::from_calendar_date(1970, time::Month::January, 1).unwrap();
        let days = (value - epoch).whole_days();
        Date(days as isize)
    }
}

#[cfg(feature = "time")]
impl From<time::OffsetDateTime> for Date {
    fn from(value: time::OffsetDateTime) -> Self {
        Date::from(value.date())
    }
}

#[cfg(feature = "time")]
impl From<time::PrimitiveDateTime> for Date {
    fn from(value: time::PrimitiveDateTime) -> Self {
        Date::from(value.date())
    }
}

impl std::fmt::Debug for Date {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let (y, m, d) = self.ymd();
        write!(f, "Date({y:04}-{m:02}-{d:02})")
    }
}

/// Utility functions that extend all supported date types and provide methods
/// on them to directly query holiday information.
pub trait DateExt: Into<Date> + Clone {
    // Most date(time) types are trivial to Clone

    /// Returns an iterator of holidays that are observed on this date in
    /// specified `country` (or many of them).
    /// 
    /// This is an alias for [`holiday::get`] method, see that method for more
    /// details.
    /// 
    /// [`holiday::get`]: crate::get
    fn holidays<CountryIter>(&self, country: impl Into<CountrySelection<CountryIter>>) -> crate::Iter
    where
        CountryIter: IntoIterator<Item = crate::Country>, {
        crate::get(country, self.clone())
    }

    /// Returns `true` if any holidays are observed on this date in specified
    /// `country` (or many of them).
    /// 
    /// This is an alias for [`holiday::is_holiday`] method, see that method for
    /// more details.
    /// 
    /// [`holiday::is_holiday`]: crate::is_holiday
    fn is_holiday<CountryIter>(&self, country: impl Into<CountrySelection<CountryIter>>) -> bool
    where
        CountryIter: IntoIterator<Item = crate::Country> {
        crate::is_holiday(country, self.clone())
    }
}

impl DateExt for std::time::SystemTime {}

#[cfg(feature = "chrono")]
impl DateExt for chrono::NaiveDate {}
#[cfg(feature = "chrono")]
impl DateExt for chrono::DateTime<chrono::Utc> {}
#[cfg(feature = "chrono")]
impl DateExt for chrono::DateTime<chrono::Local> {}

/// Error returned when conversion to/from another date format can't be
/// performed because one has larger span than the other and conversion would
/// cause an overflow.
#[derive(Debug, thiserror::Error, PartialEq, Eq)]
#[error("Date is too large for conversion")]
pub struct DateConversionError;
