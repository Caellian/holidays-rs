use chrono::NaiveDate;
use holidays::Country;

fn main() -> anyhow::Result<()> {
    let d = NaiveDate::from_ymd_opt(2022, 1, 1).expect("Invalid date");
    println!(
        "Is {d} a holiday in Japan? Answer is {}",
        holidays::contains(Country::JP, d)
    );

    println!("{:?}", holidays::get(Country::JP, d).next().unwrap());

    Ok(())
}
