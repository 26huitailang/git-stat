use std::{error::Error, fs::File, path::Path};

use chrono;
use csv::Writer;
use git2::{DiffOptions, Repository};

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

fn main() {
    let url = "https://github.com/26huitailang/yogo.git";
    let into = "./repos/yogo";

    let repo = match clone_or_open_repo(url, into) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to clone repository: {}", e),
    };

    println!("Cloned repository to: {}", repo.path().display());

    // 切换到指定分支
    let branch = repo.find_branch("main", git2::BranchType::Local).unwrap();
    // TODO: reset hard 便于统计

    println!("branch: {}", branch.name().unwrap().unwrap());
    // 遍历这个branch上所有commit
    let mut rev = repo.revwalk().unwrap();
    rev.set_sorting(git2::Sort::TIME).unwrap();
    rev.push_head().unwrap();

    let mut csv_data: Vec<Vec<String>> = Vec::new();

    for oid in rev {
        let commit = repo.find_commit(oid.unwrap()).unwrap();
        // get commit status
        // let status = commit.status().unwrap();
        let parent = match commit.parent(0) {
            Ok(tree) => tree,
            Err(_) => continue,
        };
        let tree = commit.tree().unwrap();
        let parent_tree = parent.tree().unwrap();
        let mut diff_options = DiffOptions::new();
        let diff = repo
            .diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diff_options))
            .unwrap();
        let stats = diff.stats().unwrap();

        // 时间戳转换
        let time = commit.time().seconds();
        // ts to datetime
        let datetime = chrono::DateTime::from_timestamp(time, 0).unwrap();

        println!(
            "commit: {} | {} | {} | {} | {}",
            datetime.format("%Y-%m-%d %H:%M:%S"),
            commit.id(),
            commit.author(),
            stats.insertions(),
            stats.deletions()
        );
        // append to data

        // _item 加入 csv_data
        csv_data.push(
            [
                datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
                commit.id().to_string(),
                commit.committer().to_string(),
                commit.message().unwrap().to_string(),
                stats.insertions().to_string(),
                stats.deletions().to_string(),
            ]
            .to_vec(),
        );
    }
    // 构造csv header
    let csv_header = vec![
        "date".to_string(),
        "commit_id".to_string(),
        "author".to_string(),
        "message".to_string(),
        "insertions".to_string(),
        "deletions".to_string(),
    ];
    write_csv(FILENAME, csv_header, csv_data).unwrap();
}
