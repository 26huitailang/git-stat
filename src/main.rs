use std::{borrow::Borrow, error::Error, ffi::CString, fs::File, path::Path};

use chrono;
use csv::Writer;
use git2::{DiffOptions, Pathspec, Repository};

mod config;

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

fn repo_parse(repo_conf: config::Repo) -> Result<(), Box<dyn Error>> {
    let url = repo_conf.url.as_str();
    let repo_name = url.split("/").last().unwrap().split(".").nth(0).unwrap();
    let into = format!("./repos/{}", repo_name);

    let repo = match clone_or_open_repo(url, into.as_str()) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to clone repository: {}", e),
    };

    println!("Cloned repository to: {}", repo.path().display());

    // 构造csv header
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
    let author_list = repo_conf.get_authors();
    // 切换到指定分支
    for b in repo_conf.branchs {
        let branch_name = b.as_str();
        // FIXME:这里branch没有切换
        // let head = repo.head().unwrap();
        // let oid = head.target().unwrap();
        // let commit = repo.find_commit(oid).unwrap();
        // let branch = repo.branch(branch_name, &commit, false).unwrap();
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

            // _item 加入 csv_data
            csv_data.push(
                [
                    datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
                    branch_name.to_string(),
                    commit.id().to_string(),
                    commit.committer().name().unwrap_or("").to_string(),
                    commit.message().unwrap().to_string(),
                    stats.insertions().to_string(),
                    stats.deletions().to_string(),
                ]
                .to_vec(),
            );
        }
    }

    write_csv(FILENAME, csv_header, csv_data)
}

fn main() {
    let conf = config::new(".git-stat.yml");
    for repo in conf.repos {
        repo_parse(repo).unwrap()
    }
}
