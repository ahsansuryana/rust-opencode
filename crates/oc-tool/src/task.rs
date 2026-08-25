//! Ported dari packages/opencode/src/tool/task.ts — TaskTool for subagent spawning.

use std::sync::Arc;

use serde_json::{json, Value};

use oc_session::prompt::SubagentContext;

/// TaskTool — spawns subagent sessions.
/// Ported from: tool/task.ts:25-303
pub struct TaskTool {
    pub spawner: Arc<dyn oc_session::prompt::SubagentSpawner>,
    pub max_depth: usize,
}

impl TaskTool {
    pub fn new(spawner: Arc<dyn oc_session::prompt::SubagentSpawner>, max_depth: usize) -> Self {
        Self { spawner, max_depth }
    }

    pub fn definition() -> Value {
        json!({
            "name": "task",
            "description": "Launch a new agent to handle complex, multistep tasks autonomously.",
            "parameters": {
                "type": "object",
                "required": ["description", "prompt", "subagent_type"],
                "properties": {
                    "description": { "type": "string", "description": "A short (3-5 words) description of the task" },
                    "prompt": { "type": "string", "description": "The task for the agent to perform autonomously" },
                    "subagent_type": { "type": "string", "description": "The type of specialized agent to use" },
                    "task_id": { "type": "string", "description": "Resume a previous task (optional)" },
                    "command": { "type": "string", "description": "Command that triggered this task (optional)" },
                    "background": { "type": "boolean", "description": "Run asynchronously in background (optional)" }
                }
            }
        })
    }

    pub fn execute(
        &self,
        args: &Value,
        ctx: &crate::Context,
    ) -> Result<crate::ExecuteResult, crate::ToolError> {
        let description = args["description"].as_str().unwrap_or("");
        let prompt = args["prompt"].as_str().unwrap_or("");
        let subagent_type = args["subagent_type"].as_str().unwrap_or("general");

        if prompt.is_empty() {
            return Err(crate::ToolError::Message("prompt is required".to_string()));
        }

        let sub_ctx = SubagentContext {
            parent_session_id: ctx.session_id.clone(),
            directory: ctx.directory.clone(),
            worktree: ctx.worktree.clone(),
            model_provider_id: String::new(),
            model_id: String::new(),
        };

        let output = self
            .spawner
            .spawn_subagent(subagent_type, prompt, &sub_ctx, 0)
            .map_err(crate::ToolError::Message)?;

        Ok(crate::ExecuteResult {
            title: format!("subagent:{subagent_type} — {description}"),
            metadata: json!({"subagent": subagent_type, "description": description}),
            output,
        })
    }
}
