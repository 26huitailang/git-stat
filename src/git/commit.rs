use crate::{config, git};
use chrono::{DateTime, Local, TimeZone};
use git2::{Cred, Diff, DiffOptions, RemoteCallbacks, Repository};
use std::collections::HashMap;
use std::error::Error;
use std::path::Path;
fn clone_or_open_repo(
    url: &str,
    into: &str,
    repo_conf: config::Repo,
) -> Result<Repository, git2::Error> {
    if Path::new(into).exists() {
        Repository::open(into)
    } else {
        let mut callbacks = RemoteCallbacks::new();
        callbacks.credentials(|_url, _username_from_url, _allowed_types| {
            Cred::userpass_plaintext(repo_conf.username(), repo_conf.password())
        });
        // Prepare fetch options.
        let mut fo = git2::FetchOptions::new();
        fo.remote_callbacks(callbacks);

        let mut builder = git2::build::RepoBuilder::new();
        builder.fetch_options(fo);
        builder.clone(url, into.as_ref())
    }
}
#[derive(Debug, Clone)]
pub struct CommitInfo {
    pub repo_name: String,
    pub datetime: Option<DateTime<Local>>,
    pub branch: String,
    pub commit_id: String,
    pub author: String,
    pub message: String,
    pub insertions: usize,
    pub deletions: usize,
}

impl CommitInfo {
    fn new(
        repo_name: String,
        datetime: Option<DateTime<Local>>,
        branch: String,
        commit_id: String,
        author: String,
        message: String,
        insertions: usize,
        deletions: usize,
    ) -> Self {
        CommitInfo {
            repo_name,
            datetime,
            branch,
            commit_id,
            author,
            message,
            insertions,
            deletions,
        }
    }

    pub fn format_datetime(&self) -> String {
        match &self.datetime {
            None => "".to_string(),
            Some(datetime) => datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitInfoVec {
    pub commit_info_vec: Vec<CommitInfo>,
    pub sum_vec: Vec<CommitInfo>,
}

impl CommitInfoVec {
    pub fn new(commit_info_vec: Vec<CommitInfo>) -> Self {
        CommitInfoVec {
            commit_info_vec,
            sum_vec: vec![],
        }
    }

    pub fn sum_insertions_deletions_by_branch_and_author(&mut self) {
        let mut grouped_data: HashMap<(String, String, String), (usize, usize)> = HashMap::new();

        // Group by branch and author, summing insertions
        for commit_info in &self.commit_info_vec {
            let key = (
                commit_info.repo_name.clone(),
                commit_info.branch.clone(),
                commit_info.author.clone(),
            );
            let (insertions, deletions) = grouped_data.entry(key.clone()).or_insert((0, 0));
            *insertions += commit_info.insertions;
            *deletions += commit_info.deletions;
        }

        // Convert the grouped data back into CommitInfo instances for sum_vec
        self.sum_vec = grouped_data
            .into_iter()
            .map(|((repo_name, branch, author), total_lines)| CommitInfo {
                repo_name,
                datetime: None,
                branch: branch.clone(),
                commit_id: "".to_string(),
                author,
                message: format!("Total lines: +{} -{}", total_lines.0, total_lines.1),
                insertions: total_lines.0,
                deletions: total_lines.1, // Assuming we're not tracking total deletions in this context
            })
            .collect();
    }

    pub fn filter_by_date(
        &mut self,
        since: Option<DateTime<Local>>,
        until: Option<DateTime<Local>>,
    ) {
        println!(
            "filter_by_date before count: {}",
            self.commit_info_vec.len()
        );
        self.commit_info_vec = self
            .commit_info_vec
            .clone()
            .into_iter()
            .filter(|commit_info| {
                commit_info.datetime.is_some()
                    && (since.is_none() || commit_info.datetime.unwrap() >= since.unwrap())
                    && (until.is_none() || commit_info.datetime.unwrap() <= until.unwrap())
            })
            .collect();
        println!("filter_by_date after count: {}", self.commit_info_vec.len());
    }
}

pub fn repo_parse(repo_conf: config::Repo) -> Result<Vec<CommitInfo>, Box<dyn Error>> {
    let url = repo_conf.url.as_str();
    let into = format!("./repos/{}", repo_conf.repo_name());

    let repo = match clone_or_open_repo(url, into.as_str(), repo_conf.clone()) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to clone repository: {}", e),
    };

    println!("Cloned repository to: {}", repo.path().display());

    let mut commit_data: Vec<CommitInfo> = Vec::new();
    let author_list = repo_conf.get_authors();
    // 切换到指定分支
    for b in &repo_conf.branches {
        let branch_name = b.as_str();
        let _ = repo.find_remote("origin").expect("remote not found");
        let args = git::Args {
            arg_remote: Some("origin".to_string()),
            arg_branch: Some(branch_name.to_string()),
        };
        git::pull(&args, &repo, repo_conf.username(), repo_conf.password())
            .expect("git pull failed");
        // print current branch  and commit ref
        let _ = repo.set_head(format!("refs/remotes/origin/{}", branch_name).as_str());
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                // For some reason the force is required to make the working directory actually get updated
                // I suspect we should be adding some logic to handle dirty working directory states
                // but this is just an example so maybe not.
                .force(),
        ))
        .expect("checkout failed");

        let (object, reference) = repo
            .revparse_ext(repo.head().unwrap().name().unwrap())
            .expect("branch not found");
        repo.checkout_tree(&object, None).expect("checkout failed");
        match reference {
            Some(gref) => {
                let _ = repo.set_head(gref.name().unwrap());
                println!("Checked out branch: {}", gref.name().unwrap());
            }
            None => {
                println!("this is a commit");
                // 返回错误
                return Err(Box::new(git2::Error::from_str("branch not found")));
            }
        }

        println!("branch: {}", branch_name);
        // 遍历这个branch上所有commit
        let mut rev = repo.revwalk().unwrap();
        rev.set_sorting(git2::Sort::TIME).unwrap();
        rev.push_head().unwrap();

        let mut diff_options = DiffOptions::new();
        // include suffix file type
        for pathspec_str in &repo_conf.pathspec {
            // warn: 这里 !framework 要写到其他类似 *.go 前面，否则不生效
            diff_options.pathspec(pathspec_str);
            println!("pathspec set: {}", pathspec_str);
        }
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
            let diff: Diff;

            match commit.parent(0) {
                Ok(parent) => {
                    let parent_tree = parent.tree().unwrap();
                    diff = repo
                        .diff_tree_to_tree(Some(&parent_tree), Some(&tree), Some(&mut diff_options))
                        .unwrap();
                }
                Err(_) => {
                    println!("no parent, try none diff");
                    diff = repo
                        .diff_tree_to_tree(None, Some(&tree), Some(&mut diff_options))
                        .unwrap();
                }
            };
            let stats = diff.stats().unwrap();
            if stats.files_changed() == 0 {
                println!("no files changed, skipppp");
                continue;
            }

            // 时间戳转换
            let time = commit.time().seconds();
            let datetime = Local::timestamp_opt(&Local, time, 0).unwrap();

            let author = match repo_conf.map_alias_to_name(commit.author().name().clone().unwrap())
            {
                Some(name) => name,
                None => {
                    println!("no author name found, use author name");
                    commit.author().name().unwrap().to_string()
                }
            };
            println!(
                "repo: {} commit: {} | {} | {} | {} | +{} | -{} | {}",
                repo_conf.repo_name(),
                datetime.format("%Y-%m-%d %H:%M:%S"),
                branch_name,
                commit.id(),
                author,
                stats.insertions(),
                stats.deletions(),
                commit.summary().unwrap_or(""),
            );
            // append to data

            let commit_row = CommitInfo::new(
                repo_conf.repo_name().to_string(),
                datetime.into(),
                branch_name.to_string(),
                commit.id().to_string(),
                author,
                commit.message().unwrap().to_string(),
                stats.insertions(),
                stats.deletions(),
            );
            commit_data.push(commit_row);
        }
    }

    return Ok(commit_data);
}
