use std::{collections::HashMap, path::Path};

use axum::http::Method;
use serde::{Deserialize, Deserializer};

pub type ProjectRoutes = HashMap<String, Vec<ProjectRoute>>;

#[derive(Debug, Deserialize)]
pub struct ProjectConfig {
    pub name: String,
    pub routes: ProjectRoutes,
}

impl ProjectConfig {
    pub fn load(filename: impl AsRef<Path>) -> anyhow::Result<Self> {
        let content = std::fs::read_to_string(filename)?;
        Ok(serde_yaml::from_str(&content)?)
    }
}

#[derive(Debug, Deserialize, PartialEq)]
pub struct ProjectRoute {
    #[serde(deserialize_with = "deserialize_method")]
    pub method: Method,
    pub handler: String,
}

fn deserialize_method<'de, D>(deserializer: D) -> Result<Method, D::Error>
where
    D: Deserializer<'de>,
{
    let s = String::deserialize(deserializer)?;
    Method::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom)?;
    match s.to_uppercase().as_str() {
        "GET" => Ok(Method::GET),
        "POST" => Ok(Method::POST),
        "PUT" => Ok(Method::PUT),
        "DELETE" => Ok(Method::DELETE),
        "PATCH" => Ok(Method::PATCH),
        "HEAD" => Ok(Method::HEAD),
        "OPTIONS" => Ok(Method::OPTIONS),
        "CONNECT" => Ok(Method::CONNECT),
        "TRACE" => Ok(Method::TRACE),
        _ => Err(serde::de::Error::custom("invalid method")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deserialize_config_should_work() -> anyhow::Result<()> {
        let s = r#"---
name: dino-test
routes:
  /api/hello/id:
    - method: GET
      handler: hello1
    - method: POST
      handler: hello2
  /api/name/id:
    - method: GET
      handler: hello3
    - method: POST
      handler: hello4
"#;
        let config: ProjectConfig = serde_yaml::from_str(s)?;
        assert_eq!(config.name, "dino-test");
        assert_eq!(
            config.routes["/api/hello/id"],
            vec![
                ProjectRoute {
                    method: Method::GET,
                    handler: "hello1".to_string()
                },
                ProjectRoute {
                    method: Method::POST,
                    handler: "hello2".to_string()
                }
            ]
        );
        Ok(())
    }
}
