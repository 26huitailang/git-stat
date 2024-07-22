use serde::Deserialize;
use std::fs::File;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub authors: Vec<Author>,
    pub repos: Vec<Repo>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Repo {
    pub url: String,
    username: Option<String>,
    password: Option<String>,
    pub branches: Vec<String>,
    pub pathspec: Vec<String>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Author {
    pub name: String,
    pub alias: Vec<String>,
}

impl Config {
    pub fn new(filename: &str) -> Config {
        let reader = File::open(filename).unwrap();
        let config: Config = serde_yaml::from_reader(reader).unwrap();
        config
    }
}

// 为Struct实现一个方法
impl Repo {
    pub fn repo_name(&self) -> &str {
        self.url
            .split("/")
            .last()
            .unwrap()
            .split(".")
            .nth(0)
            .unwrap()
    }

    pub fn username(&self) -> &str {
        if self.username.is_some() {
            return self.username.as_ref().unwrap();
        } else {
            return "";
        }
    }
    pub fn password(&self) -> &str {
        if self.password.is_some() {
            return self.password.as_ref().unwrap();
        } else {
            return "";
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_config() {
        let content = r##"authors:
  - name: 26huitailang
    alias: [peterChen]
repos:
  - url: https://github.com/26huitailang/yogo.git
    username:
    password:
    branches: [main]
    pathspec:
      - "*.go"
      - "!framework"
      - "!vendor"
"##;
        let config: Config = serde_yaml::from_str(content).unwrap();
        println!("{:?}", config);
        assert_eq!(
            config.repos[0].url,
            "https://github.com/26huitailang/yogo.git"
        );
        assert_eq!(config.repos[0].branches[0], "main");
        assert_eq!(config.authors[0].name, "26huitailang");
        assert_eq!(config.authors[0].alias, &["peterChen"]);
        assert_eq!(config.repos[0].pathspec[0], "*.go");
        assert_eq!(config.repos[0].pathspec[1], "!framework");
        assert_eq!(config.repos[0].pathspec[2], "!vendor");
    }
}
