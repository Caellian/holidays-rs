use chrono::NaiveDate;
use holidays::{Country, Query};

fn main() -> anyhow::Result<()> {
    let s = NaiveDate::from_ymd_opt(2022, 1, 1).expect("Invalid date");
    let u = NaiveDate::from_ymd_opt(2023, 1, 1).expect("Invalid date");

    for holiday in holidays::query(Query::country(Country::JP).and(Query::date_range(s..u))).map(|h| h.date) {
        println!("{holiday:?}",);
    }

    Ok(())
}
