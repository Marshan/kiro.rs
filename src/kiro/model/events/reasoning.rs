//! 推理内容事件
//!
//! 处理 reasoningContentEvent 类型的事件

use serde::{Deserialize, Serialize};

use crate::kiro::parser::error::ParseResult;
use crate::kiro::parser::frame::Frame;

use super::base::EventPayload;

/// 推理内容事件
///
/// 包含 AI 助手的流式推理思考内容
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReasoningContentEvent {
    /// 推理内容片段
    #[serde(default)]
    pub text: String,

    /// 捕获其他未使用的字段，确保反序列化兼容性
    #[serde(flatten)]
    #[serde(skip_serializing)]
    #[allow(dead_code)]
    extra: serde_json::Value,
}

impl EventPayload for ReasoningContentEvent {
    fn from_frame(frame: &Frame) -> ParseResult<Self> {
        frame.payload_as_json()
    }
}

impl Default for ReasoningContentEvent {
    fn default() -> Self {
        Self {
            text: String::new(),
            extra: serde_json::Value::Null,
        }
    }
}

impl ReasoningContentEvent {
    /// 创建推理内容事件
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            extra: serde_json::Value::Null,
        }
    }
}

