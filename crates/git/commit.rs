use config;
use chrono::{DateTime, Local, TimeZone};
use git2::{Cred, Diff, DiffOptions, RemoteCallbacks, Repository};
use log::{debug, info, trace, warn};
use serde::{Serialize, Serializer};
use std::error::Error;
use std::io::{Cursor, Write};
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

pub fn serialize_dt<S>(dt: &Option<DateTime<Local>>, serializer: S) -> Result<S::Ok, S::Error>
where
    S: Serializer,
{
    match dt {
        Some(dt) => {
            let s = dt.format("%Y-%m-%d %H:%M:%S").to_string();
            serializer.serialize_str(s.as_str())
        }
        None => {
            let s = "".to_string();
            serializer.serialize_str(&s)
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct CommitInfo {
    pub repo: String,
    #[serde(
        serialize_with = "serialize_dt",
        skip_serializing_if = "Option::is_none"
    )]
    pub date: Option<DateTime<Local>>,
    pub branch: String,
    pub commit_id: String,
    pub author: String,
    pub message: String,
    pub insertions: usize,
    pub deletions: usize,
}

impl CommitInfo {
    fn new(
        repo: String,
        date: Option<DateTime<Local>>,
        branch: String,
        commit_id: String,
        author: String,
        message: String,
        insertions: usize,
        deletions: usize,
    ) -> Self {
        CommitInfo {
            repo,
            date,
            branch,
            commit_id,
            author,
            message,
            insertions,
            deletions,
        }
    }

    pub fn format_datetime(&self) -> String {
        match &self.date {
            None => "".to_string(),
            Some(datetime) => datetime.format("%Y-%m-%d %H:%M:%S").to_string(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct CommitInfoVec {
    pub commit_info_vec: Vec<CommitInfo>,
}

impl CommitInfoVec {
    pub fn new(commit_info_vec: Vec<CommitInfo>) -> Self {
        CommitInfoVec { commit_info_vec }
    }

    pub fn file_cursor(&self) -> Result<Cursor<Vec<u8>>, std::io::Error> {
        let mut w = csv::Writer::from_writer(Cursor::new(Vec::new()));
        w.write_record(&[
            "repo".to_string(),
            "date".to_string(),
            "branch".to_string(),
            "commit_id".to_string(),
            "author".to_string(),
            "message".to_string(),
            "insertions".to_string(),
            "deletions".to_string(),
        ])
        .unwrap();

        for commit_info in &self.commit_info_vec {
            w.write_record(&[
                commit_info.repo.to_string(),
                commit_info.format_datetime(),
                commit_info.branch.to_string(),
                commit_info.commit_id.to_string(),
                commit_info.author.to_string(),
                commit_info.message.to_string(),
                commit_info.insertions.to_string(),
                commit_info.deletions.to_string(),
            ])
            .unwrap();
        }
        let mut cursor = w.into_inner().unwrap();
        cursor.flush()?;
        Ok(cursor)
    }
}

pub fn repo_parse(
    repo_conf: &config::Repo,
    update: bool,
) -> Result<Vec<CommitInfo>, Box<dyn Error>> {
    let url = repo_conf.url.as_str();
    let into = format!("./repos/{}", repo_conf.repo_name());

    let repo = match clone_or_open_repo(url, into.as_str(), repo_conf.clone()) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to clone repository: {}", e),
    };

    info!("clone/open repository: {}", repo.path().display());

    let mut commit_data: Vec<CommitInfo> = Vec::new();
    // 切换到指定分支
    for b in &repo_conf.branches {
        let branch_name = b.as_str();
        let _ = repo.find_remote("origin").expect("remote not found");
        let args = crate::repo::Args {
            arg_remote: Some("origin".to_string()),
            arg_branch: Some(branch_name.to_string()),
        };

        if update {
            crate::repo::pull(&args, &repo, repo_conf.username(), repo_conf.password())
                .expect("git pull failed");
        }
        // print current branch  and commit ref
        let _ = repo.set_head(format!("refs/remotes/origin/{}", branch_name).as_str());
        repo.checkout_head(Some(
            git2::build::CheckoutBuilder::default()
                // For some reason the force is required to make the working directory actually get updated
                // I suspect we should be adding some logic to handle dirty working directory states
                // but this is just an example so maybe not.
                .force(),
        ))
        .expect(format!("checkout failed: {}/{}", repo_conf.repo_name(), branch_name).as_str());

        let (object, reference) = repo
            .revparse_ext(repo.head().unwrap().name().unwrap())
            .expect("branch not found");
        repo.checkout_tree(&object, None).expect("checkout failed");
        match reference {
            Some(gref) => {
                let _ = repo.set_head(gref.name().unwrap());
                debug!("Checked out branch: {}", gref.name().unwrap());
            }
            None => {
                debug!("this is a commit");
                // 返回错误
                return Err(Box::new(git2::Error::from_str("branch not found")));
            }
        }

        info!("walk branch: {}/{}", repo_conf.repo_name(), branch_name);
        // 遍历这个branch上所有commit
        let mut rev = repo.revwalk().unwrap();
        rev.set_sorting(git2::Sort::TIME).unwrap();
        rev.push_head().unwrap();

        let mut diff_options = DiffOptions::new();
        // include suffix file type
        for pathspec_str in &repo_conf.pathspec {
            // warn: 这里 !framework 要写到其他类似 *.go 前面，否则不生效
            diff_options.pathspec(pathspec_str);
            debug!("pathspec set: {}", pathspec_str);
        }
        for oid in rev {
            let commit = repo.find_commit(oid.unwrap()).unwrap();
            if commit.parent_count() > 1 {
                debug!(
                    "commit has more than one parent, maybe merge commit, skip: {}",
                    commit.id()
                );
                continue;
            }

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
                    debug!("no parent, try none diff");
                    diff = repo
                        .diff_tree_to_tree(None, Some(&tree), Some(&mut diff_options))
                        .unwrap();
                }
            };
            let stats = diff.stats().unwrap();
            if stats.files_changed() == 0 {
                debug!("no files changed, skip: {}", commit.id());
                continue;
            }

            // 时间戳转换
            let time = commit.time().seconds();
            let datetime = Local::timestamp_opt(&Local, time, 0).unwrap();

            let author = commit.author().name().unwrap().to_string();
            // let author = match repo_conf.map_alias_to_name(commit.author().name().clone().unwrap())
            // {
            //     Some(name) => name,
            //     None => {
            //         println!("no author name found, use author name");
            //         commit.author().name().unwrap().to_string()
            //     }
            // };
            trace!(
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
            let cmt_msg = match commit.message() {
                Some(msg) => msg.to_string(),
                None => {
                    warn!("no commit message found, use empty string: {}", commit.id());
                    "".to_string()
                }
            };
            let commit_row = CommitInfo::new(
                repo_conf.repo_name().to_string(),
                datetime.into(),
                branch_name.to_string(),
                commit.id().to_string(),
                author,
                cmt_msg,
                stats.insertions(),
                stats.deletions(),
            );
            commit_data.push(commit_row);
        }
    }

    return Ok(commit_data);
}
