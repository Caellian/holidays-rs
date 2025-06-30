use chrono::NaiveDate;
use holidays::Country;

fn main() {
    let date = NaiveDate::from_ymd_opt(2022, 1, 1).expect("Invalid date");
    
    let holidays = holidays::get_holidays(Country::JP, date).map(|h| h.date::<NaiveDate>().unwrap());
    for holiday in holidays {
        println!("{holiday:?}",);
    }
}
