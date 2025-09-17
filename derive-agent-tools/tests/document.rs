use std::collections::HashMap;

use aws_smithy_types::{Document, Number};
use derive_agent_tools::AgentTool;
use serde::Deserialize;

#[derive(Debug, PartialEq, AgentTool, Deserialize)]
#[tool(description = "Doc round trip")]
struct RoundTrip {
    #[tool(required)]
    answer: i32,
    #[tool(description = "toggle")]
    toggle: Option<bool>,
}

#[test]
#[cfg(all(feature = "bedrock", feature = "serde-json"))]
fn parses_document_payloads() {
    let mut inner = HashMap::new();
    inner.insert("answer".to_string(), Document::Number(Number::PosInt(42)));
    inner.insert("toggle".to_string(), Document::Bool(true));
    let doc = Document::Object(inner);

    let parsed: RoundTrip = (&doc).try_into().expect("parseable");
    assert_eq!(
        parsed,
        RoundTrip {
            answer: 42,
            toggle: Some(true)
        }
    );
}
