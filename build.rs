use csv::StringRecord;
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Display,
    fs::File,
    hash::Hash,
    io::{BufReader, BufWriter, Write},
    path::PathBuf,
    str::FromStr,
};

// Make sure to also update ./gen.py years range
// These numbers should be more conservative to reduce compile time
const DEFAULT_MIN_YEAR: isize = 2000;
const DEFAULT_MAX_YEAR: isize = 2035;

#[derive(PartialEq, Eq, PartialOrd, Ord)]
struct Country {
    index: u16,
    code: String,
    name: String,
}

pub fn is_country_enabled(code: &str) -> bool {
    let feature = format!("CARGO_FEATURE_{code}");
    std::env::var(&feature).is_ok()
}

impl Display for Country {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Country::{}", self.code)
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
struct Date {
    year: i16,
    month: u8,
    day: u8,
    day_index: isize,
}

pub const fn ymd_as_isize(y: isize, m: isize, d: isize) -> isize {
    // Source: https://howardhinnant.github.io/date_algorithms.html
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

    days_since_julian - 719163
}

impl Date {}
impl FromStr for Date {
    type Err = ();

    fn from_str(date: &str) -> Result<Self, Self::Err> {
        let mut date = date.split("-");
        let year = date.next().ok_or(())?.parse().map_err(|_| ())?;
        let month = date.next().ok_or(())?.parse().map_err(|_| ())?;
        let day = date.next().ok_or(())?.parse().map_err(|_| ())?;
        Ok(Date {
            year,
            month,
            day,
            day_index: ymd_as_isize(year as isize, month as isize, day as isize),
        })
    }
}

#[derive(PartialEq, Eq)]
struct FullSpec<'a>(&'a Country, Date);
impl<'a> Hash for FullSpec<'a> {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.index.hash(state);
        self.1.day_index.hash(state);
    }
}
impl<'a> phf_shared::PhfHash for FullSpec<'a> {
    fn phf_hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.index.hash(state);
        self.1.day_index.hash(state);
    }
}
impl<'a> phf_shared::FmtConst for FullSpec<'a> {
    fn fmt_const(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Point({}, Date({}))", self.0, self.1.day_index)
    }
}

#[derive(PartialEq, Eq)]
struct Holiday<'a> {
    country: &'a Country,
    date: Date,
    name: String,
}

fn parse_holiday_row<'a>(
    row: StringRecord,
    countries: &'a HashMap<String, Country>,
) -> Option<Holiday<'a>> {
    let mut it = row.iter().map(String::from);

    let code = it.next().expect("invalid row in holidays.csv");
    let country = countries.get(&code)?;

    Some(Holiday {
        country,
        date: {
            let date = it.next().expect("invalid row in holidays.csv");
            date.parse().expect("invalid date format in holidays.csv")
        },
        name: it.next().expect("invalid row in holidays.csv"),
    })
}

impl<'a> PartialOrd for Holiday<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl<'a> Ord for Holiday<'a> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.date
            .cmp(&other.date)
            .then(self.country.cmp(other.country))
            .then(self.name.cmp(&other.name))
    }
}

fn gen_country_enum_decl<'a, W: Write, C: Iterator<Item = &'a Country>>(
    out: &mut W,
    countries: C,
) -> std::io::Result<()> {
    let mut reverse_lookup = phf_codegen::Map::<&str>::new();

    out.write_all(b"declare_countries![\n")?;
    for c in countries {
        writeln!(out, "{0}: \"{0}\" \"{1}\" {2},", c.code, c.name, c.index)?;
        reverse_lookup.entry(&c.code, format!("Country::{}", c.code));
    }
    out.write_all(b"];\n")?;

    write!(
        out,
        "pub(crate) static CODE_TO_COUNTRY: phf::Map<&'static str, Country> = {}",
        reverse_lookup.build()
    )
    .unwrap();
    writeln!(out, ";").unwrap();

    Ok(())
}

fn gen_data_tables<W: Write>(out: &mut W, holidays: &[Holiday]) -> std::io::Result<()> {
    let mut year_lookup = BTreeMap::new();
    let mut country_lookup = BTreeMap::new();
    let mut exact_lookup = phf_codegen::Map::<FullSpec>::new();

    out.write_all(b"pub(crate) static DATA: &[Holiday] = &[\n")?;
    for (i, h) in holidays.iter().enumerate() {
        writeln!(
            out,
            "crate::Holiday {{ code: {}, date: Date({}), name: \"{}\" }},",
            h.country, h.date.day_index, h.name
        )?;
        year_lookup.entry(h.date.year).or_insert(i);
        country_lookup
            .entry(&h.country.index)
            .or_insert(Vec::new())
            .push(i);
        exact_lookup.entry(FullSpec(h.country, h.date), i.to_string());
    }
    out.write_all(b"];\n")?;

    let min_year = *year_lookup.first_entry().unwrap().key();
    let max_year = *year_lookup.last_entry().unwrap().key();
    writeln!(out, "pub(crate) const DATA_MIN_YEAR: isize = {min_year};")?;
    writeln!(out, "pub(crate) const DATA_MAX_YEAR: isize = {max_year};")?;

    out.write_all(b"pub(crate) static YEAR_JUMP_TABLE: &[usize] = &[")?;
    let mut index = 0;
    for y in min_year..max_year {
        index = *year_lookup.get(&y).unwrap_or(&index);
        write!(out, "{index},")?;
    }
    out.write_all(b"];\n")?;

    let min_country = **country_lookup.first_entry().unwrap().key();
    let max_country = **country_lookup.last_entry().unwrap().key();
    out.write_all(b"pub(crate) static COUNTRY_JUMP_TABLE: &[&[usize]] = &[")?;
    for ci in min_country..max_country {
        let indices = country_lookup
            .get(&ci)
            .map(|it| it.as_slice())
            .unwrap_or(&[]);

        let indices = indices
            .iter()
            .map(|it| it.to_string())
            .fold("".to_string(), |acc, it| acc + it.as_str() + ",");
        writeln!(out, "&[{indices}],")?;
    }
    out.write_all(b"];\n")?;

    write!(
        out,
        "pub(crate) static DATA_MAP: phf::Map<Point, usize> = {}",
        exact_lookup.build()
    )
    .unwrap();
    writeln!(out, ";").unwrap();

    Ok(())
}

fn main() {
    let root = PathBuf::from(std::env::var("CARGO_MANIFEST_DIR").unwrap());
    let countries_path = root.join("countries.csv");
    let mut countries: Vec<Country> = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::new(match File::open(&countries_path) {
            Ok(it) => it,
            Err(_) => {
                panic!("missing {}", countries_path.display())
            }
        }))
        .records()
        .filter_map(Result::ok)
        .map(|it| {
            let mut it = it.iter().map(String::from);
            (
                it.next().expect("invalid row countries.csv"),
                it.next().expect("invalid row countries.csv"),
            )
        })
        .filter(|(code, _)| is_country_enabled(code))
        .map(|(code, name)| Country {
            index: 0,
            code,
            name,
        })
        .collect();
    countries.sort_by(|a, b| a.code.cmp(&b.code));
    countries.iter_mut().enumerate().for_each(|(i, it)| {
        it.index = i as u16;
    });

    let out_dir = PathBuf::from(&std::env::var("OUT_DIR").unwrap());
    let countries_out = out_dir.join("decl_countries.rs");
    let mut countries_out =
        BufWriter::new(File::create(countries_out).expect("unable to create decl_countries.rs"));
    gen_country_enum_decl(&mut countries_out, countries.iter()).unwrap();

    let countries: HashMap<String, Country> = countries
        .into_iter()
        .map(|it| (it.code.clone(), it))
        .collect();

    let min_req_year = std::env::var("HOLIDAYS_MIN_YEAR")
        .map(|it| it.parse().unwrap_or(DEFAULT_MIN_YEAR))
        .unwrap_or(DEFAULT_MIN_YEAR) as i16;
    let max_req_year = std::env::var("HOLIDAYS_MAX_YEAR")
        .map(|it| it.parse().unwrap_or(DEFAULT_MAX_YEAR))
        .unwrap_or(DEFAULT_MAX_YEAR) as i16;

    let holidays_path = root.join("holidays.csv");
    let holidays: Vec<Holiday> = csv::ReaderBuilder::new()
        .has_headers(true)
        .from_reader(BufReader::new(match File::open(&holidays_path) {
            Ok(it) => it,
            Err(_) => {
                panic!("missing {}", holidays_path.display())
            }
        }))
        .records()
        .filter_map(Result::ok)
        .filter_map(|row| parse_holiday_row(row, &countries))
        .skip_while(|it| it.date.year < min_req_year)
        .take_while(|it| it.date.year <= max_req_year)
        .collect();

    let holidays_out = out_dir.join("holiday_data.rs");
    let mut holidays_out =
        BufWriter::new(File::create(holidays_out).expect("unable to create holiday_data.rs"));
    gen_data_tables(&mut holidays_out, &holidays).unwrap();
}
