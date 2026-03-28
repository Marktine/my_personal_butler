use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Serialize, Clone)]
pub struct GeminiRequest {
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "systemInstruction")]
    pub system_instruction: Option<Content>,
    pub contents: Vec<Content>,
    pub tools: Option<Vec<Tool>>,
}

#[derive(Serialize, Clone)]
pub struct Tool {
    pub function_declarations: Vec<FunctionDeclaration>,
}

#[derive(Serialize, Clone)]
pub struct FunctionDeclaration {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Content {
    pub role: String,
    pub parts: Vec<Part>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Part {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "functionCall")]
    pub function_call: Option<FunctionCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(rename = "functionResponse")]
    pub function_response: Option<FunctionResponse>,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionCall {
    pub name: String,
    pub args: Value,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct FunctionResponse {
    pub name: String,
    pub response: Value,
}

#[derive(Deserialize, Debug)]
pub struct UsageMetadata {
    #[serde(rename = "totalTokenCount")]
    pub total_token_count: Option<usize>,
}

#[derive(Deserialize, Debug)]
pub struct GeminiResponse {
    pub candidates: Option<Vec<Candidate>>,
    #[serde(rename = "usageMetadata")]
    pub usage_metadata: Option<UsageMetadata>,
}

#[derive(Deserialize, Debug)]
pub struct Candidate {
    pub content: Option<Content>,
}
