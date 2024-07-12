use chrono::{DateTime, Local, NaiveDate};
use clap::builder::PossibleValuesParser;
use clap::Parser;
use csv::Writer;
use git::commit::CommitInfoVec;
use itertools::Itertools;
use std::{error::Error, fs::File, path::Path};
mod config;
mod git;
mod ui;
use polars::prelude::*;

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
    POLAR,
}

impl OutputType {
    // fn as_str(&self) -> &'static str {
    //     match self {
    //         OutputType::CSV => "csv",
    //         OutputType::TABLE => "table",
    //     }
    // }

    fn from_str(s: &str) -> Option<Self> {
        match s {
            "csv" => Some(OutputType::CSV),
            "table" => Some(OutputType::TABLE),
            "polar" => Some(OutputType::POLAR),
            _ => None,
        }
    }
}

trait Output {
    fn output(&self) -> Result<(), Box<dyn Error>>;
}

struct PolarOutput {
    df: DataFrame,
}

impl PolarOutput {
    fn new(df: DataFrame) -> Self {
        PolarOutput { df }
    }
}

impl Output for PolarOutput {
    fn output(&self) -> Result<(), Box<dyn Error>> {
        println!("{}", self.df);
        Ok(())
    }
}

struct CsvOutput {
    filename: String,
    df: DataFrame,
}

impl CsvOutput {
    fn new(filename: String, df: DataFrame) -> Self {
        CsvOutput { filename, df }
    }
}

impl Output for CsvOutput {
    fn output(&self) -> Result<(), Box<dyn Error>> {
        let mut file = std::fs::File::create(&self.filename).unwrap();
        let mut m_df = self.df.clone();
        Ok(CsvWriter::new(&mut file).finish(&mut m_df).unwrap())
        // let csv_header = vec![
        //     "repo".to_string(),
        //     "date".to_string(),
        //     "branch".to_string(),
        //     "commit_id".to_string(),
        //     "author".to_string(),
        //     "message".to_string(),
        //     "insertions".to_string(),
        //     "deletions".to_string(),
        // ];
        // let mut csv_data: Vec<Vec<String>> = Vec::new();
        // for commit_info in &self.data {
        //     csv_data.push(
        //         [
        //             commit_info.repo_name.to_string(),
        //             commit_info.format_datetime(),
        //             commit_info.branch.to_string(),
        //             commit_info.commit_id.to_string(),
        //             commit_info.author.to_string(),
        //             commit_info.message.to_string(),
        //             commit_info.insertions.to_string(),
        //             commit_info.deletions.to_string(),
        //         ]
        //         .to_vec(),
        //     );
        // }
        // write_csv(&self.filename, csv_header, csv_data)
    }
}

fn vec_commit_to_data(commit_vec: Vec<git::commit::CommitInfo>) -> Vec<Data> {
    (0..commit_vec.len())
        .map(|i| Data {
            repo: commit_vec[i].repo.to_string(),
            date: commit_vec[i].format_datetime(),
            branch: commit_vec[i].branch.to_string(),
            author: commit_vec[i].author.to_string(),
            insertions: commit_vec[i].insertions.to_string(),
            deletions: commit_vec[i].deletions.to_string(),
        })
        .sorted_by(|a, b| b.date.cmp(&a.date))
        .collect_vec()
}

struct TableOutput {
    df: DataFrame,
}

impl TableOutput {
    fn new(df: DataFrame) -> Self {
        TableOutput { df }
    }
}

fn convert_df_to_data_vec(df: DataFrame) -> Vec<Data> {
    let mut d = df
        .select(["repo", "branch", "author", "insertions", "deletions"])
        .unwrap();

    let mut j = Vec::<u8>::new();
    JsonWriter::new(&mut j)
        .with_json_format(JsonFormat::Json)
        .finish(&mut d)
        .unwrap();
    let rows = serde_json::from_slice::<Vec<Data>>(&j).unwrap();
    rows
}

impl Output for TableOutput {
    fn output(&self) -> Result<(), Box<dyn Error>> {
        let data_vec = convert_df_to_data_vec(self.df.clone());
        ui::tui::run(data_vec)
    }
}

/// Simple program to greet a person
#[derive(Parser, Debug, Clone)]
#[command(version, about, long_about = None)]
struct Args {
    #[arg(
        short = 'F',
        long = "format",
        value_parser = PossibleValuesParser::new(["csv", "table", "polar"]),
        default_value = "polar",
        help = "output format"
    )]
    format: String,

    #[arg(
        long = "detail",
        help = "keep detail csv file or not, e.g. --detail output.csv"
    )]
    detail: Option<String>,

    #[arg(long = "source", help = "do not parse repo again, use SOURCE directly")]
    source: Option<String>,

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

fn get_output(output_type: OutputType, df: DataFrame) -> Box<dyn Output> {
    match output_type {
        OutputType::TABLE => Box::new(TableOutput::new(df)),
        OutputType::CSV => Box::new(CsvOutput::new(String::from("report.csv"), df)),
        OutputType::POLAR => Box::new(PolarOutput::new(df)),
    }
}

fn load_df_from_csv(filename: String) -> DataFrame {
    let csv = LazyCsvReader::new(filename)
        .with_try_parse_dates(true)
        .with_has_header(true)
        .finish()
        .unwrap();
    csv.collect().unwrap()
}

pub struct MyDataFrame {
    df: DataFrame,
}

impl MyDataFrame {
    pub fn new(df: DataFrame) -> Self {
        MyDataFrame { df }
    }
    pub fn summary(
        &self,
        since: Option<DateTime<Local>>,
        until: Option<DateTime<Local>>,
    ) -> DataFrame {
        let q = self.df.clone().lazy();

        let mut filter_expr = lit(true);

        if let Some(since) = since {
            let since_expr = lit(since.naive_local());
            filter_expr = filter_expr.and(col("date").gt_eq(since_expr));
        };
        if let Some(until) = until {
            let until_expr = lit(until.naive_local());
            filter_expr = filter_expr.and(col("date").lt_eq(until_expr));
        };

        q.filter(filter_expr)
            .select(vec![
                col("repo"),
                col("branch"),
                col("author"),
                col("insertions"),
                col("deletions"),
            ])
            .group_by(["repo", "branch", "author"])
            .agg([col("*").sum()])
            .sort(["repo", "branch"], SortMultipleOptions::default())
            .collect()
            .unwrap()
    }
}

fn main() {
    let args = Args::parse();
    let conf = config::Config::new(".git-stat.yml");
    let mut repo_data: Vec<CommitInfo> = vec![];
    // TODO: if --source detail.csv 指定了，则从 detail.csv 中读取数据
    // 否则从配置文件分析获取
    let df = match args.source {
        Some(source) => load_df_from_csv(source),
        None => {
            for repo in conf.repos {
                let data = git::commit::repo_parse(repo).unwrap();
                repo_data.extend(data);
            }
            let json_str = serde_json::to_string(&repo_data).unwrap();
            let file = std::io::Cursor::new(json_str);
            CsvReadOptions::default()
                .with_has_header(true)
                .map_parse_options(|s| s.with_try_parse_dates(true))
                .into_reader_with_file_handle(file)
                .finish()
                .unwrap()
        }
    };
    // TODO: use polar csv writer save raw data
    let mut info_vec = CommitInfoVec::new(repo_data);
    if args.detail.is_some() {
        let detail_file = args.detail.clone().unwrap_or("detail.csv".to_string());
        println!("detail csv file: {}", detail_file);
        CsvOutput::new(detail_file, df.clone())
            .output()
            .expect("detail csv output failed");
    }
    // TODO summary
    let my_df = MyDataFrame::new(df);
    let summ = my_df.summary(args.since, args.until);

    // TODO df -> 格式转换如何实现，csv/table等需要的格式

    // TODO: 计算都用polars实现

    let out_type = OutputType::from_str(args.format.as_str()).unwrap();
    get_output(out_type, summ).output().expect("output failed");
}
