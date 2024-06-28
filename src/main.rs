use std::path::Path;

use git2::{DiffOptions, Repository};

fn clone_or_open_repo(url: &str, into: &str) -> Result<Repository, git2::Error> {
    if Path::new(into).exists() {
        Repository::open(into)
    } else {
        Repository::clone(url, into)
    }
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

        println!(
            "commit: {} {} {} {}",
            commit.id(),
            commit.author(),
            stats.insertions(),
            stats.deletions()
        );
    }
}
