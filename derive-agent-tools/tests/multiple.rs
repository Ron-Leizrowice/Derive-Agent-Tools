use derive_agent_tools::AgentTool;
use serde::Deserialize;

#[allow(dead_code)]
#[derive(AgentTool, Deserialize)]
#[tool(name = "first", description = "First tool")]
struct FirstTool {
    #[tool(required)]
    a: i32,
}

#[allow(dead_code)]
#[derive(AgentTool, Deserialize)]
#[tool(description = "Second tool")]
struct SecondTool {
    #[tool(required)]
    flag: bool,
}

#[test]
fn derives_do_not_conflict() {
    assert_eq!(FirstTool::tool_name(), "first");
    assert_eq!(SecondTool::tool_name(), "SecondTool");

    #[cfg(feature = "bedrock")]
    {
        let _spec = FirstTool::tool_spec();
        let _spec = SecondTool::tool_spec();
    }
}
