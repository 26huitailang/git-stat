use git2::Repository;

fn main() {
    let url = "https://github.com/26huitailang/yogo.git";
    let into = "./repos/yogo";
    if std::path::Path::new(into).exists() {
        match std::fs::remove_dir_all(into) {
            Ok(_) => {}
            Err(e) => {
                panic!("{}", e);
            }
        }
    } else {
        println!("path not exists: {}", into);
    }
    let repo = match Repository::clone(url, into) {
        Ok(repo) => repo,
        Err(e) => panic!("Failed to clone repository: {}", e),
    };

    println!("Cloned repository to: {}", repo.path().display());

    // 切换到指定分支
    let branch = repo.find_branch("main", git2::BranchType::Local).unwrap();

    println!("branch: {}", branch.name().unwrap().unwrap());
    // 遍历这个branch上所有commit
    let mut rev = repo.revwalk().unwrap();
    rev.set_sorting(git2::Sort::TIME).unwrap();
    rev.push_head().unwrap();

    for oid in rev {
        let commit = repo.find_commit(oid.unwrap()).unwrap();
        println!("commit: {}", commit.id());
    }
}