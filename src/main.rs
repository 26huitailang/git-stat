use std::{error::Error, fs::File, path::Path};

use chrono::{DateTime, Utc};
use csv::Writer;
use git2::{DiffOptions, Repository};
use itertools::Itertools;
use ratatui::widgets::{Row, StatefulWidget, Table};

mod config;
mod tui;
use tui::*;

const FILENAME: &str = "repo.csv";
fn clone_or_open_repo(url: &str, into: &str) -> Result<Repository, git2::Error> {
    if Path::new(into).exists() {
        Repository::open(into)
    } else {
        Repository::clone(url, into)
    }
}
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

#[derive(Debug)]
struct CommitInfo {
    datetime: DateTime<Utc>,
    branch: String,
    commit_id: String,
    committer: String,
    message: String,
    insertions: usize,
    deletions: usize,
}

impl CommitInfo {
    fn new(
        datetime: DateTime<Utc>,
        branch: String,
        commit_id: String,
        committer: String,
        message: String,
        insertions: usize,
        deletions: usize,
    ) -> Self {
        CommitInfo {
            datetime,
            branch,
            commit_id,
            committer,
            message,
            insertions,
            deletions,
        }
    }
}
fn repo_parse(repo_conf: config::Repo) -> Result<Vec<CommitInfo>, Box<dyn Error>> {
    let url = repo_conf.url.as_str();
    let repo_name = url.split("/").last().unwrap().split(".").nth(0).unwrap();
    let into = format!("./repos/{}", repo_name);

    let repo = match clone_or_open_repo(url, into.as_str()) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to clone repository: {}", e),
    };

    println!("Cloned repository to: {}", repo.path().display());

    // 构造csv header
    let mut commit_data: Vec<CommitInfo> = Vec::new();
    let author_list = repo_conf.get_authors();
    // 切换到指定分支
    for b in repo_conf.branchs {
        let branch_name = b.as_str();
        // 分支切换
        let (object, reference) = repo.revparse_ext(branch_name).expect("branch not found");
        repo.checkout_tree(&object, None).expect("checkout failed");
        match reference {
            Some(gref) => {
                repo.set_head(gref.name().unwrap());
                println!("Checked out branch: {}", gref.name().unwrap());
            }
            None => {
                println!("this is a commit");
                // 返回错误
                return Err(Box::new(git2::Error::from_str("branch not found")));
            }
        }

        // TODO: reset hard 便于统计
        println!("branch: {}", branch_name);
        // 遍历这个branch上所有commit
        let mut rev = repo.revwalk().unwrap();
        rev.set_sorting(git2::Sort::TIME).unwrap();
        rev.push_head().unwrap();

        let mut diff_options = DiffOptions::new();
        // include suffix file type
        for pathspec_str in vec!["!framework", "*.go"] {
            // let c_pathspec = CString::new(pathspec_str).expect("failed to create CString");
            diff_options.pathspec(pathspec_str);
        }
        // TODO try walk_hide_callback
        for oid in rev {
            let commit = repo.find_commit(oid.unwrap()).unwrap();
            // commit author 不在 authors中跳过
            if !author_list.contains(&commit.author().name().unwrap().to_string()) {
                println!(
                    "author: {} not in authors, skip",
                    commit.author().name().unwrap()
                );
                continue;
            }
            // get commit status
            // let status = commit.status().unwrap();
            let parent = match commit.parent(0) {
                Ok(tree) => tree,
                Err(_) => continue,
            };
            let tree = commit.tree().unwrap();
            let parent_tree = parent.tree().unwrap();
            let diff = repo
                .diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diff_options))
                .unwrap();
            let stats = diff.stats().unwrap();

            // 时间戳转换
            let time = commit.time().seconds();
            // ts to datetime
            let datetime = chrono::DateTime::from_timestamp(time, 0).unwrap();

            println!(
                "commit: {} | {} | {} | {} | {} | {} | {}",
                datetime.format("%Y-%m-%d %H:%M:%S"),
                branch_name,
                commit.id(),
                commit.author().name().unwrap_or(""),
                stats.insertions(),
                stats.deletions(),
                commit.summary().unwrap_or(""),
            );
            // append to data

            let commit_row = CommitInfo::new(
                datetime,
                branch_name.to_string(),
                commit.id().to_string(),
                commit.committer().name().unwrap_or("").to_string(),
                commit.message().unwrap().to_string(),
                stats.insertions(),
                stats.deletions(),
            );
            commit_data.push(commit_row);
        }
    }

    return Ok(commit_data);
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

fn csv_output(data: Vec<CommitInfo>) -> Result<(), Box<dyn Error>> {
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
                commit_info.datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
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

fn generate_data_vec(commit_vec: Vec<CommitInfo>) -> Vec<Data> {
    (0..commit_vec.len())
        .map(|i| {
            Data {
                date: commit_vec[i].datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
                branch: commit_vec[i].branch.to_string(),
                committer: commit_vec[i].committer.to_string(),
                insertions: commit_vec[i].insertions.to_string(),
                deletions: commit_vec[i].deletions.to_string(),
            }
        })
        .sorted_by(|a, b| b.date.cmp(&a.date))
        .collect_vec()
}

fn table_output(data: Vec<CommitInfo>) -> Result<(), Box<dyn Error>> {
    let data_vec = generate_data_vec(data);
    tui::run(data_vec)
}

fn main() {
    let conf = config::new(".git-stat.yml");
    for repo in conf.repos {
        let repo_data = repo_parse(repo).unwrap();
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
