use std::{error::Error, fs::File, path::Path};
use csv::Writer;
use itertools::Itertools;
use ratatui::widgets::{StatefulWidget};

mod config;
mod ui;
mod git;

use ui::*;
use crate::config::Repo;
use crate::ui::data::Data;

const FILENAME: &str = "repo.csv";

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

fn csv_output(data: Vec<git::commit::CommitInfo>) -> Result<(), Box<dyn Error>> {
    let csv_header = vec![
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
                commit_info.format_datetime(),
                commit_info.branch.to_string(),
                commit_info.commit_id.to_string(),
                commit_info.committer.to_string(),
                commit_info.message.to_string(),
                commit_info.insertions.to_string(),
                commit_info.deletions.to_string(),
            ]
                .to_vec(),
        );
    }
    write_csv(FILENAME, csv_header, csv_data)
}

fn vec_commit_to_data(commit_vec: Vec<git::commit::CommitInfo>) -> Vec<Data> {
    (0..commit_vec.len())
        .map(|i| {
            Data {
                date: commit_vec[i].format_datetime(),
                branch: commit_vec[i].branch.to_string(),
                committer: commit_vec[i].committer.to_string(),
                insertions: commit_vec[i].insertions.to_string(),
                deletions: commit_vec[i].deletions.to_string(),
            }
        })
        .sorted_by(|a, b| b.date.cmp(&a.date))
        .collect_vec()
}

fn table_output(data: Vec<git::commit::CommitInfo>) -> Result<(), Box<dyn Error>> {
    let mut commit_vec = git::commit::CommitInfoVec::new(data);
    commit_vec.sum_insertions_deletions_by_branch_and_committer();
    let data_vec = vec_commit_to_data(commit_vec.sum_vec);
    ui::tui::run(data_vec)
}

fn main() {
    let conf = config::new(".git-stat.yml");
    for repo in conf.repos {
        let repo_data = git::commit::repo_parse(repo).unwrap();
        match OutputType::from_str(conf.output.as_str()).expect("output not match") {
            OutputType::CSV => {
                csv_output(repo_data).expect("csv output failed");
            }
            OutputType::TABLE => {
                table_output(repo_data).expect("table output failed");
            }
        }
    }
}
