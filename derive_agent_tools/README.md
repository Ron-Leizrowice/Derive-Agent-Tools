# derive_agent_tools

Derive macros to define AI tools and their parameters directly from Rust structs.

- AgentTool: derive on a struct to generate an AWS Bedrock `ToolSpecification`, a JSON schema helper, and an implementation to parse Bedrock tool inputs into your struct.
- AgentToolParameter: derive on supporting types; currently provides a basic JSON schema helper. This will expand as more providers are supported.

Status: Bedrock-only today. Additional tool formats will be added over time.

## Why

Writing Bedrock tool schemas by hand is repetitive and error‑prone. `derive_agent_tools` lets you define a tool once as a Rust type and get:

- A validated JSON schema (properties + required)
- A Bedrock `ToolSpecification`
- A `TryFrom<&aws_smithy_types::Document>` impl to parse tool inputs into your struct

## Install

Add to your Cargo.toml:

```toml
[dependencies]
derive_agent_tools = "0.1"
serde = { version = "1", features = ["derive"] }
```

### Features

`derive_agent_tools` exposes two optional capabilities controlled by feature
flags. Both of them are enabled by default.

- `serde-json` – builds JSON Schema helpers and requires `serde`/`serde_json`
  at runtime.
- `bedrock` – generates AWS Bedrock `ToolSpecification` builders and pulls in
  the AWS SDK dependencies.

If you want to opt out of the AWS SDK dependencies, disable default features and
pick the subset you need:

```toml
[dependencies]
derive_agent_tools = { version = "0.1", default-features = false, features = ["serde-json"] }
serde = { version = "1", features = ["derive"] }
```

With this configuration the crate still derives tools and the JSON schema
helpers compile, but Bedrock-specific functions such as
`AgentTool::tool_spec()` are not generated.

## Usage

```rust
use derive_agent_tools::AgentTool;
use serde::Deserialize;

#[derive(AgentTool, Deserialize)]
#[tool(description = "A tool to get the weather")]
struct WeatherTool {
    #[tool(required, description = "The latitude of the location")]
    latitude: f64,
    #[tool(required, description = "The longitude of the location")]
    longitude: f64,
}

// Register tool with Bedrock
#[cfg(feature = "bedrock")]
let spec = WeatherTool::tool_spec();

// Inspect schema if desired
#[cfg(feature = "serde-json")]
let schema = WeatherTool::tool_schema_json();

// If your Agent returns a ToolUse input Document, you can parse it:
// let args: WeatherTool = (&document).try_into()?;
```

### Attributes

- Struct-level `#[tool(...)]`:
  - `name = "..."` override the tool name (defaults to struct name)
  - `description = "..."` human-friendly description
- Field-level `#[tool(...)]`:
  - `required` mark a field as required (otherwise it is optional in the schema)
  - `description = "..."` field description

### Type mapping

Basic Rust types map to JSON Schema as follows:

- `bool` -> `boolean`
- integer types -> `integer`
- `f32`, `f64` -> `number`
- `String`, `&str` -> `string`
- `Vec<T>` -> `array` (best-effort `items` type)
- `Option<T>` -> uses `T`'s type but is not marked as required
- Other types default to `object`

This mapping is intentionally minimal and conservative. It will be expanded over time.

## Bedrock Support

`tool_spec()` builds an `aws_sdk_bedrockruntime::types::ToolSpecification` using a JSON schema generated from your struct and annotations. Only Bedrock is supported at present. The crate is structured to support additional providers in the future through feature flags and provider-specific builders.

## Error Handling

- Misuse of the macros (e.g., deriving on non-structs or tuple structs) produces compile‑time errors.
- When the generated `TryFrom<&Document>` implementation fails to deserialize the payload, the error message is captured in a lightweight, per-type error struct.
