use ollama_rs::{
    generation::{
        chat::ChatMessage,
        parameters::{FormatType, JsonStructure},
    },
    history::ChatHistory,
};
use schemars::{JsonSchema, schema_for};
use serde::{Deserialize, Serialize};

use crate::{
    AppResult,
    chat::OllamaChat,
    ui::{
        AppUIRenderable, RoundedBox,
        utils::{AnsiColor, colorize_text},
    },
};

static AGENT_SYSTEM_PROMPT: &str = "You are an AI agent assisting user in complexe tasks. Based on the user request, you will generate a list of tasks then execute them one by one. After each task you will wait for user confirmation before continue to the next one. Follow step by step your plan to achieve the final goal. Do not interupt the user for anything else than confirmation at the end of each stem.";
static PLAN_GENERATION_PROMPT: &str = "Start by generate a list of tasks to achieve the user request. Each one must be clear and detailed. Output this plan as JSON object with a key 'tasks' containing an array of task objects. Each task object must have a 'name' and a 'description'. Do not output anything else than the JSON object.";

#[derive(Debug, Clone)]
pub enum AgentStatus {
    Disabled,
    Ready,
    Plan,
    Running,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentPlan {
    pub tasks: Vec<AgentTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, JsonSchema)]
pub struct AgentTask {
    name: String,
    description: String,
    completed: bool,
}

#[derive(Debug)]
pub struct AgentMode {
    status: AgentStatus,
    plan: AgentPlan,
}

impl AgentMode {
    pub fn new() -> Self {
        AgentMode {
            status: AgentStatus::Disabled,
            plan: AgentPlan { tasks: Vec::new() },
        }
    }

    pub fn init(&mut self, ollama_chat: &mut OllamaChat) {
        self.status = AgentStatus::Ready;
        self.plan.tasks.clear();

        ollama_chat.use_system_prompt = false;

        let binding = ollama_chat.get_history();
        let mut history = binding.lock().unwrap();
        history.clear();
        history.push(ChatMessage::system(AGENT_SYSTEM_PROMPT.to_string()));
    }

    pub async fn handle_prompt(
        &mut self,
        prompt: &str,
        ollama_chat: &mut OllamaChat,
    ) -> AppResult<()> {
        match self.status {
            AgentStatus::Disabled => {
                return Err("Agent mode is disabled".into());
            }
            AgentStatus::Ready | AgentStatus::Plan => {
                self.status = AgentStatus::Plan;

                let messages = match self.status {
                    AgentStatus::Plan => {
                        vec![ChatMessage::user(prompt.to_string())]
                    }
                    AgentStatus::Ready => {
                        vec![
                            ChatMessage::system(PLAN_GENERATION_PROMPT.to_string()),
                            ChatMessage::user(prompt.to_string()),
                        ]
                    }
                    _ => unreachable!(),
                };

                let json_schema = schema_for!(AgentPlan);

                let format = FormatType::StructuredJson(Box::new(JsonStructure::new_for_schema(
                    json_schema,
                )));

                dbg!(ollama_chat.get_history().lock().unwrap().messages());

                let model_output = match ollama_chat.formated_request(messages, format).await {
                    Ok(res) => res,
                    Err(e) => {
                        self.status = AgentStatus::Ready;
                        return Err(format!("Error generating agent plan: {}", e).into());
                    }
                };

                self.plan = match serde_json::from_str::<AgentPlan>(&model_output.message.content) {
                    Ok(plan) => plan,
                    Err(e) => {
                        self.status = AgentStatus::Ready;
                        return Err(format!("Error parsing agent plan: {}", e).into());
                    }
                };

                println!("{}", self.plan.render(true));
                println!(
                    "Model generate a plan, please confirm it with /agent start or provide a new prompt to update it"
                )
            }
            _ => {}
        };

        Ok(())
    }

    pub fn get_status(&self) -> &AgentStatus {
        &self.status
    }
}

impl AppUIRenderable for AgentPlan {
    fn render(&self, boxed: bool) -> String {
        let mut rendered_string = String::new();

        for (i, task) in self.tasks.iter().enumerate() {
            rendered_string.push_str(&colorize_text(
                &format!("{}. {}\n", i, task.name),
                AnsiColor::BrightGreen,
            ));
            rendered_string.push_str(&format!("{}\n", task.description));
            rendered_string.push_str(&format!(
                "Status: {}\n \n",
                if task.completed {
                    colorize_text("Completed", AnsiColor::BrightBlue)
                } else {
                    colorize_text("Pending", AnsiColor::BrightRed)
                }
            ));
        }

        if boxed {
            RoundedBox::new(
                &rendered_string,
                Some(&"Agent Plan"),
                Some(AnsiColor::BrightGreen),
                false,
            )
            .render()
        } else {
            rendered_string
        }
    }
}
