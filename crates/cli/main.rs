use config;
use config::Repo;
use git::commit::CommitInfo;
use git::commit::CommitInfoVec;
use ui::data::Data;

use chrono::{DateTime, Local, NaiveDate};
use clap::builder::PossibleValuesParser;
use clap::Parser;
use csv::Writer;
use env_logger::Env;
use polars::lazy::dsl::GetOutput;
use polars::prelude::*;
use std::collections::{HashMap, HashSet};
use std::sync::mpsc;
use std::{error::Error, fs::File, path::Path};
use std::{thread, time};

use log::{debug, error, info};

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
    let file = File::create(&filename).unwrap();
    let mut wtr = Writer::from_writer(file);

    wtr.write_record(header)?;

    for record in data {
        wtr.write_record(record)?;
    }
    wtr.flush()?;
    info!("CSV file written successfully: {:?}", filename.as_ref());

    Ok(())
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
    }
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

    #[arg(long = "no-detail", action=clap::ArgAction::SetTrue, help="do not keep detail csv file, ignore --detail if this is set")]
    no_detail: bool,

    #[arg(long = "source", help = "do not parse repo again, use SOURCE directly")]
    source: Option<String>,

    /// since date
    #[arg(long = "since", value_parser = parse_since, help = "since date, 2024-01-01")]
    since: Option<DateTime<Local>>,

    /// until date
    #[arg(long = "until", value_parser = parse_until, help = "since date, 2024-03-31")]
    until: Option<DateTime<Local>>,

    #[arg(long = "force-update", action=clap::ArgAction::SetTrue, help="pull from remote repo")]
    update: bool,
}

fn parse_since(s: &str) -> Result<DateTime<Local>, Box<std::io::Error>> {
    let since = parse_date(s, [0, 0, 0]).unwrap();
    info!("since: {}", since);
    Ok(since)
}
fn parse_until(s: &str) -> Result<DateTime<Local>, Box<std::io::Error>> {
    let until = parse_date(s, [23, 59, 59]).unwrap();
    info!("until: {}", until);
    Ok(until)
}

fn parse_date(s: &str, hms_opt: [u32; 3]) -> Result<DateTime<Local>, Box<std::io::Error>> {
    let date = match NaiveDate::parse_from_str(s, "%Y-%m-%d") {
        Ok(d) => d,
        Err(e) => {
            error!("parse date err: {}", e);
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

#[derive(Debug)]
pub struct FilterOptions {
    pub since: Option<DateTime<Local>>,
    pub until: Option<DateTime<Local>>,
    pub authors: HashMap<String, String>,
}

pub struct MyDataFrame<'a> {
    df: &'a DataFrame,
    filter_options: &'a FilterOptions,
}

impl<'a> MyDataFrame<'a> {
    pub fn new(df: &'a DataFrame, filter_options: &'a FilterOptions) -> Self {
        MyDataFrame { df, filter_options }
    }
    pub fn summary(&self) -> DataFrame {
        let mut q = self.df.clone().lazy();
        let mut allowed_authors: HashSet<String> = HashSet::new();
        for (alias, author) in &self.filter_options.authors {
            q = q.with_column(col("author").str().replace(
                lit(alias.as_str()),
                lit(author.as_str()),
                false,
            ));
            allowed_authors.insert(author.into());
        }

        let mut filter_expr = lit(true);

        if let Some(since) = self.filter_options.since {
            let since_expr = lit(since.naive_local());
            filter_expr = filter_expr.and(col("date").gt_eq(since_expr));
        };
        if let Some(until) = self.filter_options.until {
            let until_expr = lit(until.naive_local());
            filter_expr = filter_expr.and(col("date").lt_eq(until_expr));
        };

        let is_valid = move |s: Series| -> Result<Option<Series>, _> {
            let ss: Vec<_> = s
                .str()
                .unwrap()
                .into_iter()
                .map(|s| allowed_authors.contains(s.unwrap_or("")))
                .collect();
            let s = Series::new("is_valid", ss);
            Ok(Some(s))
        };
        let o = GetOutput::from_type(DataType::Boolean);
        q = q
            .with_column(
                col("author")
                    .map(is_valid, o)
                    .alias("is_valid")
                    .cast(DataType::Boolean),
            )
            .filter(col("is_valid"));

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

pub fn get_df(source: Option<String>, repos: Vec<Repo>, update: bool) -> DataFrame {
    let mut repo_data: Vec<CommitInfo> = vec![];
    let df = match source {
        Some(source) => load_df_from_csv(source),
        None => {
            let (tx, rx) = mpsc::channel();
            let mut handlers = vec![];

            for repo in repos {
                let t_sender = tx.clone();
                let t = thread::spawn(move || {
                    let repo_name = repo.repo_name();
                    info!("repo parse start: {}", repo_name);
                    let start = time::Instant::now();
                    let data = git::commit::repo_parse(&repo, update).unwrap();
                    t_sender.send(data).unwrap();
                    let duration = time::Instant::now().duration_since(start);
                    info!(
                        "repo parse done: {}, cost {}ms",
                        repo_name,
                        duration.as_millis()
                    );
                });
                handlers.push(t);
            }
            for h in handlers {
                h.join().unwrap();
            }
            drop(tx);
            info!("rx start to collect data");
            while let Ok(received) = rx.recv() {
                debug!("rx received data len {}", received.len());
                repo_data.extend(received);
            }
            info!("rx collect data done");

            let file = CommitInfoVec::new(repo_data).file_cursor().unwrap();
            CsvReadOptions::default()
                .with_has_header(true)
                .map_parse_options(|s| s.with_try_parse_dates(true))
                .into_reader_with_file_handle(file)
                .finish()
                .unwrap()
        }
    };
    df
}
fn main() {
    env_logger::Builder::from_env(Env::default().default_filter_or("info")).init();

    let args = Args::parse();
    let conf = config::Config::new(".git-stat.yml");

    let df = get_df(args.source, conf.repos, args.update);

    if !args.no_detail {
        let detail_file = args.detail.clone().unwrap_or("detail.csv".to_string());
        info!("detail csv file: {}", detail_file);
        CsvOutput::new(detail_file, df.clone())
            .output()
            .expect("detail csv output failed");
    }
    // summary by polars
    let filter_options = &mut FilterOptions {
        since: args.since,
        until: args.until,
        authors: HashMap::new(),
    };
    for author in conf.authors {
        for alias in author.alias {
            filter_options.authors.insert(alias, author.name.clone());
        }
    }
    debug!("filter options: {:?}", filter_options);
    let my_df = MyDataFrame::new(&df, filter_options);
    let summ = my_df.summary();

    let out_type = OutputType::from_str(args.format.as_str()).unwrap();
    get_output(out_type, summ).output().expect("output failed");
}
