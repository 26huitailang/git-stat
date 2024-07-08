use std::{error::Error, fs::File, path::Path};
use std::collections::HashMap;
use chrono::{DateTime, Utc};
use csv::Writer;
use git2::{Diff, DiffOptions, Repository, Tree};
use git2::build::CheckoutBuilder;
use itertools::Itertools;
use ratatui::widgets::{Row, StatefulWidget, Table};

mod config;
mod tui;
mod git;

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
struct CommitInfoVec {
    commit_info_vec: Vec<CommitInfo>,
    sum_vec: Vec<CommitInfo>,
}

impl CommitInfoVec {
    pub fn new(
        commit_info_vec: Vec<CommitInfo>,
    ) -> Self {
        CommitInfoVec {
            commit_info_vec,
            sum_vec: vec![],
        }
    }
    pub fn sum_insertions_deletions_by_branch_and_committer(&mut self) {
        let mut grouped_data: HashMap<(String, String), (usize, usize)> = HashMap::new();

        // Group by branch and committer, summing insertions
        for commit_info in &self.commit_info_vec {
            let key = (commit_info.branch.clone(), commit_info.committer.clone());
            let (insertions, deletions) = grouped_data.entry(key.clone()).or_insert((0, 0));
            *insertions += commit_info.insertions;
            *deletions += commit_info.deletions;
        }

        // Convert the grouped data back into CommitInfo instances for sum_vec
        self.sum_vec = grouped_data.into_iter()
            .map(|((branch, committer), total_lines)| CommitInfo {
                datetime: None,
                branch: branch.clone(),
                commit_id: format!("SUM_{}_{}", branch.clone(), committer), // A fabricated commit ID
                committer,
                message: format!("Total lines: +{} -{}", total_lines.0, total_lines.1),
                insertions: total_lines.0,
                deletions: total_lines.1, // Assuming we're not tracking total deletions in this context
            })
            .collect();
    }
}

#[derive(Debug)]
struct CommitInfo {
    datetime: Option<DateTime<Utc>>,
    branch: String,
    commit_id: String,
    committer: String,
    message: String,
    insertions: usize,
    deletions: usize,
}

impl CommitInfo {
    fn new(
        datetime: Option<DateTime<Utc>>,
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

    fn format_datetime(&self) -> String {
        match &self.datetime {
            None => "".to_string(),
            Some(datetime) => {datetime.format("%Y-%m-%d %H:%M:%S").to_string()}
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

    let mut commit_data: Vec<CommitInfo> = Vec::new();
    let author_list = repo_conf.get_authors();
    // 切换到指定分支
    for b in repo_conf.branches {
        let branch_name = b.as_str();
        // TODO: 分支切换，支持远端分支切换，reset hard（force）
        // TODO: origin is hard code
        let remote = repo.find_remote("origin").expect("remote not found");
        let args = git::Args {
            arg_remote: Some("origin".to_string()),
            arg_branch: Some(branch_name.to_string()),
        };
        git::pull(&args, &repo).expect("git pull failed");
        // print current branch  and commit ref
        repo.set_head(format!("refs/remotes/origin/{}", branch_name).as_str());
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                // For some reason the force is required to make the working directory actually get updated
                // I suspect we should be adding some logic to handle dirty working directory states
                // but this is just an example so maybe not.
                .force(),
        )).expect("checkout failed");

        let (object, reference) = repo.revparse_ext(repo.head().unwrap().name().unwrap()).expect("branch not found");
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
        for pathspec_str in &repo_conf.pathspec {
            // let c_pathspec = CString::new(pathspec_str).expect("failed to create CString");
            diff_options.pathspec(pathspec_str);
        }
        // TODO try walk_hide_callback
        for oid in rev {
            let commit = repo.find_commit(oid.unwrap()).unwrap();
            if commit.parent_count() > 1 {
                println!("commit has more than one parent, maybe merge commit, skip");
                continue;
            }
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
            let tree = commit.tree().unwrap();
            let mut diff: Diff;

            match commit.parent(0) {
                Ok(parent) => {
                    let parent_tree = parent.tree().unwrap();
                    diff = repo
                        .diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diff_options))
                        .unwrap();
                },
                Err(_) => {
                    println!("no parent, try none diff");
                    diff = repo
                        .diff_tree_to_tree(None, Some(&tree), Some(&mut diff_options))
                        .unwrap();

                },
            };
            let stats = diff.stats().unwrap();
            if stats.files_changed() == 0 {
                println!("no files changed, skipppp");
                continue
            }

            // 时间戳转换
            let time = commit.time().seconds();
            // ts to datetime
            let datetime = chrono::DateTime::from_timestamp(time, 0).unwrap();

            println!(
                "commit: {} | {} | {} | {} | +{} | -{} | {}",
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
                datetime.into(),
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

fn generate_data_vec(commit_vec: Vec<CommitInfo>) -> Vec<Data> {
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

fn table_output(data: Vec<CommitInfo>) -> Result<(), Box<dyn Error>> {
    let mut commit_vec = CommitInfoVec::new(data);
    commit_vec.sum_insertions_deletions_by_branch_and_committer();
    let data_vec = generate_data_vec(commit_vec.sum_vec);
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
