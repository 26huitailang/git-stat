use serde::Deserialize;
use std::fs::File;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub output: Vec<String>,
    pub repos: Vec<Repo>,
}

#[derive(Debug, Deserialize)]
pub struct Repo {
    pub url: String,
    pub branchs: Vec<String>,
    pub authors: Vec<Author>,
    pub pathspec: Vec<String>,
}

#[derive(Debug, Deserialize)]
pub struct Author {
    pub name: String,
    pub alias: Vec<String>,
}

pub fn new(filename: &str) -> Config {
    let reader = File::open(filename).unwrap();
    let config: Config = serde_yaml::from_reader(reader).unwrap();
    config
}

// 为Struct实现一个方法
impl Repo {
    pub fn get_authors(&self) -> Vec<String> {
        let mut authors = Vec::new();
        for author in &self.authors {
            authors.push(author.name.clone());
            authors.extend(author.alias.clone());
        }
        authors
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_config() {
        let content = r##"output: [csv]
repos:
  - url: https://github.com/26huitailang/yogo.git
    branchs: [main]
    authors:
      - name: 26huitailang
        alias: [peterChen]
    pathspec:
      - "*.go"
      - "!framework"
      - "!vendor"
"##;
        let config: Config = serde_yaml::from_str(content).unwrap();
        println!("{:?}", config);
        assert_eq!(config.output[0], "csv");
        assert_eq!(
            config.repos[0].url,
            "https://github.com/26huitailang/yogo.git"
        );
        assert_eq!(config.repos[0].branchs[0], "main");
        assert_eq!(config.repos[0].authors[0].name, "26huitailang");
        assert_eq!(config.repos[0].authors[0].alias, &["peterChen"]);
        assert_eq!(config.repos[0].pathspec[0], "*.go");
        assert_eq!(config.repos[0].pathspec[1], "!framework");
        assert_eq!(config.repos[0].pathspec[2], "!vendor");
    }
}