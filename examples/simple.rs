use chrono::NaiveDate;
use holidays::Country;

fn main() {
    let d = NaiveDate::from_ymd_opt(2022, 1, 1).expect("Invalid date");
    println!(
        "Is {d} a holiday in Japan? Answer is {}",
        holidays::is_holiday(Country::JP, d)
    );

    println!("{:?}", holidays::get_holidays(Country::JP, d).next().unwrap());
}
