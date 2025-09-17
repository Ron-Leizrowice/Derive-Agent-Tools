//! derive-agent-tools
//!
//! Single-install facade crate that re-exports the derive macros and provides
//! hidden re-exports used by macro expansions. Users only add this crate.
//!
//! Features
//! - `serde-json` (default): enables JSON schema helpers
//! - `bedrock` (default): enables AWS Bedrock ToolSpecification helpers
//!
//! Example
//! ```
//! use derive_agent_tools::AgentTool;
//! use serde::Deserialize;
//!
//! #[derive(AgentTool, Deserialize)]
//! #[tool(description = "A tool to get the weather")]
//! struct WeatherTool {
//!     #[tool(required, description = "The latitude of the location")]
//!     latitude: f64,
//!     #[tool(required, description = "The longitude of the location")]
//!     longitude: f64,
//! }
//!
//! #[cfg(feature = "bedrock")]
//! let _spec = WeatherTool::tool_spec();
//! #[cfg(feature = "serde-json")]
//! let _schema = WeatherTool::tool_schema_json();
//! ```

pub use derive_agent_tools_macros::{AgentTool, AgentToolParameter};

#[doc(hidden)]
pub mod __macro_support {
    #[cfg(feature = "bedrock")]
    pub use aws_sdk_bedrockruntime;
    #[cfg(feature = "bedrock")]
    pub use aws_smithy_types;
    #[cfg(feature = "serde-json")]
    pub use serde;
    #[cfg(feature = "serde-json")]
    pub use serde_json;
}
