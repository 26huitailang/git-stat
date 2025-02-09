use serde::{Deserialize, Deserializer, Serialize};

fn deserialize_string<'de, D>(deserializer: D) -> Result<String, D::Error>
where
    D: Deserializer<'de>,
{
    let value: serde_json::Value = Deserialize::deserialize(deserializer)?;
    if let serde_json::Value::String(s) = value {
        Ok(s)
    } else if let serde_json::Value::Number(s) = value {
        Ok(s.to_string())
    } else {
        Err(serde::de::Error::custom("Expected string|number"))
    }
}

fn default_str() -> String {
    String::new()
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Data {
    #[serde(default = "default_str")]
    pub repo: String,
    #[serde(default = "default_str")]
    pub date: String,
    pub branch: String,
    pub author: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub insertions: String,
    #[serde(deserialize_with = "deserialize_string")]
    pub deletions: String,
}

impl Data {
    pub const fn ref_array(&self) -> [&String; 6] {
        [
            &self.repo,
            &self.date,
            &self.branch,
            &self.author,
            &self.insertions,
            &self.deletions,
        ]
    }

    pub fn repo(&self) -> &str {
        &self.repo
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
