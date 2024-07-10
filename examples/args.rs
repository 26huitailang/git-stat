use chrono::{DateTime, Local, NaiveDate};
use clap::Parser;

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// keep detail csv file or not
    #[arg(long = "detail")]
    detail: Option<String>,

    /// since date
    #[arg(long = "since", value_parser = parse_since, help = "since date, 2024-01-01")]
    since: Option<DateTime<Local>>,

    /// until date
    #[arg(long = "until", value_parser = parse_until, help = "since date, 2024-03-31")]
    until: Option<DateTime<Local>>,
}
fn parse_since(s: &str) -> Result<DateTime<Local>, Box<std::io::Error>> {
    parse_date(s, [0, 0, 0])
}
fn parse_until(s: &str) -> Result<DateTime<Local>, Box<std::io::Error>> {
    parse_date(s, [23, 59, 59])
}

fn parse_date(s: &str, hms_opt: [u32; 3]) -> Result<DateTime<Local>, Box<std::io::Error>> {
    println!("{}", s);
    let date = match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        Ok(d) => {
            println!("since: {}", d);
            d
        }
        Err(e) => {
            println!("{}", e);
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidData,
                "Invalid date format",
            )));
        }
    };
    let d = date.and_hms_opt(hms_opt[0], hms_opt[1], hms_opt[2]);
    Ok(d.unwrap().and_local_timezone(Local).unwrap())
}

fn main() {
    let args = Args::parse();
    println!("since {}", args.since.unwrap());
    match args.until {
        Some(u) => println!("until {}", u),
        None => println!("until None"),
    }
}
