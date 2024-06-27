use std::path::Path;

use git2::Repository;

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
        println!("commit: {} {}", commit.id(), commit.message().unwrap());
    }
}
