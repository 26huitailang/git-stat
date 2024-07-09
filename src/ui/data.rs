pub struct Data {
    pub repo_name: String,
    pub date: String,
    pub branch: String,
    pub author: String,
    pub insertions: String,
    pub deletions: String,
}

impl Data {
    pub const fn ref_array(&self) -> [&String; 6] {
        [&self.repo_name, &self.date, &self.branch, &self.author, &self.insertions, &self.deletions]
    }

    pub fn repo_name(&self) -> &str {
        &self.repo_name
    }

    pub fn date(&self) -> &str {
        &self.date
    }

    pub fn branch(&self) -> &str {
        &self.branch
    }

    pub fn author(&self) -> &str {
        &self.author
    }

    pub fn insertions(&self) -> &str {
        &self.insertions
    }

    pub fn deletions(&self) -> &str {
        &self.deletions
    }
}
