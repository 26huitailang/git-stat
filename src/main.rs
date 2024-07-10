use chrono::{DateTime, Local, NaiveDate};
use clap::builder::PossibleValuesParser;
use clap::error::ErrorFormatter;
use clap::Parser;
use csv::Writer;
use fakeit::address::info;
use git::commit::CommitInfoVec;
use itertools::Itertools;
use std::default;
use std::fmt::Display;
use std::io::Stderr;
use std::{error::Error, fs::File, path::Path};
mod config;
mod git;
mod ui;

use crate::git::commit::CommitInfo;
use crate::ui::data::Data;

/// 写入csv文件
///
/// # 参数
/// * `filename` - 文件名
/// * `header` - csv文件头
/// * `data` - csv文件数据
pub fn write_csv<P: AsRef<Path>>(
    filename: P,
    header: Vec<String>,
    data: Vec<Vec<String>>,
) -> Result<(), Box<dyn Error>> {
    let file = File::create(filename).unwrap();
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(header)?;

    for record in data {
        wtr.write_record(record)?;
    }
    wtr.flush()?;
    println!("CSV file written successfully!");

    Ok(())
    // let lines = data.len();
    // println!("data has been written to {:?} {}", filename, lines);
}

enum OutputType {
    CSV,
    TABLE,
}

impl OutputType {
    fn as_str(&self) -> &'static str {
        match self {
            OutputType::CSV => "csv",
            OutputType::TABLE => "table",
        }
    }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "csv" => Some(OutputType::CSV),
            "table" => Some(OutputType::TABLE),
            _ => None,
        }
    }
}

trait Output {
    fn output(&self, data: Vec<CommitInfo>) -> Result<(), Box<dyn Error>>;
}

struct CsvOutput {
    filename: String,
}

impl Output for CsvOutput {
    fn output(&self, data: Vec<CommitInfo>) -> Result<(), Box<dyn Error>> {
        let csv_header = vec![
            "repo".to_string(),
            "date".to_string(),
            "branch".to_string(),
            "commit_id".to_string(),
            "author".to_string(),
            "message".to_string(),
            "insertions".to_string(),
            "deletions".to_string(),
        ];
        let mut csv_data: Vec<Vec<String>> = Vec::new();
        for commit_info in data {
            csv_data.push(
                [
                    commit_info.repo_name.to_string(),
                    commit_info.format_datetime(),
                    commit_info.branch.to_string(),
                    commit_info.commit_id.to_string(),
                    commit_info.author.to_string(),
                    commit_info.message.to_string(),
                    commit_info.insertions.to_string(),
                    commit_info.deletions.to_string(),
                ]
                .to_vec(),
            );
        }
        write_csv(&self.filename, csv_header, csv_data)
    }
}

fn vec_commit_to_data(commit_vec: Vec<git::commit::CommitInfo>) -> Vec<Data> {
    (0..commit_vec.len())
        .map(|i| Data {
            repo_name: commit_vec[i].repo_name.to_string(),
            date: commit_vec[i].format_datetime(),
            branch: commit_vec[i].branch.to_string(),
            author: commit_vec[i].author.to_string(),
            insertions: commit_vec[i].insertions.to_string(),
            deletions: commit_vec[i].deletions.to_string(),
        })
        .sorted_by(|a, b| b.date.cmp(&a.date))
        .collect_vec()
}

struct TableOutput;
impl Output for TableOutput {
    fn output(&self, data: Vec<CommitInfo>) -> Result<(), Box<dyn Error>> {
        let data_vec = vec_commit_to_data(data);
        ui::tui::run(data_vec)
    }
}

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short = 'F',
        long = "format",
        value_parser = PossibleValuesParser::new(["csv", "table"]),
        default_value = "csv",
        help = "output format"
    )]
    format: String,

    #[arg(
        long = "detail",
        help = "keep detail csv file or not, e.g. --detail output.csv"
    )]
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
    let conf = config::Config::new(".git-stat.yml");
    let mut repo_data: Vec<CommitInfo> = vec![];
    for repo in conf.repos {
        let data = git::commit::repo_parse(repo).unwrap();
        repo_data.extend(data);
    }
    // save raw data
    let mut info_vec = CommitInfoVec::new(repo_data);
    if args.detail.is_some() {
        let detail_file = args.detail.clone().unwrap_or("detail.csv".to_string());
        println!("detail csv file: {}", detail_file);
        CsvOutput {
            filename: detail_file,
        }
        .output(info_vec.commit_info_vec.clone())
        .expect("detail csv output failed");
    }

    info_vec.filter_by_date(args.since, args.until);

    info_vec.sum_insertions_deletions_by_branch_and_author();

    match OutputType::from_str(args.format.as_str()).expect("output not match") {
        OutputType::CSV => {
            CsvOutput {
                filename: "report.csv".to_string(),
            }
            .output(info_vec.sum_vec)
            .expect("csv output failed");
        }
        OutputType::TABLE => {
            TableOutput {}
                .output(info_vec.sum_vec)
                .expect("table output failed");
        }
    }
}
