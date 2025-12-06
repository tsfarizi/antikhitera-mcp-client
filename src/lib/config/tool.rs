use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq, ToSchema)]
pub struct ToolConfig {
    pub name: String,
    pub description: Option<String>,
    #[serde(default)]
    pub server: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub(crate) enum RawTool {
    Name(String),
    Detailed {
        name: String,
        description: Option<String>,
        #[serde(default)]
        server: Option<String>,
    },
}

impl From<RawTool> for ToolConfig {
    fn from(value: RawTool) -> Self {
        match value {
            RawTool::Name(name) => Self {
                name,
                description: None,
                server: None,
            },
            RawTool::Detailed {
                name,
                description,
                server,
            } => Self {
                name,
                description,
                server,
            },
        }
    }
}
