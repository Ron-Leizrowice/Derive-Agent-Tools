use derive_agent_tools::AgentTool;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(AgentTool, Deserialize)]
#[tool(description = "A test tool")]
struct TTool {
    #[tool(required, description = "a")]
    a: i32,
    #[tool(description = "b")]
    b: Option<String>,
    c: Vec<f64>,
}

#[test]
fn schema_has_shape() {
    let schema = TTool::tool_schema_json();
    let obj = schema.as_object().expect("object schema");
    assert_eq!(obj.get("type").unwrap().as_str(), Some("object"));
    let props = obj.get("properties").unwrap().as_object().unwrap();
    assert_eq!(
        props.get("a").unwrap().get("type").unwrap().as_str(),
        Some("integer")
    );
    assert_eq!(
        props.get("b").unwrap().get("type").unwrap().as_str(),
        Some("string")
    );
    assert_eq!(
        props.get("c").unwrap().get("type").unwrap().as_str(),
        Some("array")
    );
    let required = obj.get("required").unwrap().as_array().unwrap();
    assert!(required.iter().any(|v| v.as_str() == Some("a")));
}
