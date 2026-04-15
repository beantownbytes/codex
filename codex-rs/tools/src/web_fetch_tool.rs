use crate::JsonSchema;
use crate::ResponsesApiTool;
use crate::ToolSpec;
use std::collections::BTreeMap;

pub fn create_web_fetch_tool() -> ToolSpec {
    let properties = BTreeMap::from([
        (
            "url".to_string(),
            JsonSchema::string(Some("The URL to fetch content from.".to_string())),
        ),
        (
            "max_length".to_string(),
            JsonSchema::number(Some(
                "Maximum number of characters to return. Defaults to 100000.".to_string(),
            )),
        ),
    ]);

    ToolSpec::Function(ResponsesApiTool {
        name: "web_fetch".to_string(),
        description: "Fetch the content of a URL and return it as text.".to_string(),
        strict: false,
        defer_loading: None,
        parameters: JsonSchema::object(
            properties,
            Some(vec!["url".to_string()]),
            Some(false.into()),
        ),
        output_schema: None,
    })
}
