use serde::Deserialize;

#[derive(Debug, Deserialize)]
struct Config {
    output: Vec<String>,
    repos: Vec<Repo>,
}

#[derive(Debug, Deserialize)]
struct Repo {
    url: String,
    branchs: Vec<String>,
    authors: Vec<Author>,
    pathspec: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct Author {
    name: String,
    alias: Vec<String>,
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
