use std::{
    fs::{read_to_string, write},
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use serde_json::{Number, Value};

use crate::{
    AppResult, ConfigFile, get_config_path,
    ui::{
        AppUIRenderable, RoundedBox,
        input::{MenuChoice, menu_selection, text_input},
        utils::{AnsiColor, colorize_text},
    },
};

static CATEGORIES: [(&str, &[&str]); 3] = [
    (
        "Model",
        &[
            "model_name",
            "show_thinking",
            "model_seed",
            "model_temperature",
            "model_system_prompt",
        ],
    ),
    ("Tool Calls", &["verbose_tool_calls", "tool_confirmation"]),
    ("Configuration", &["auto_save_config", "config_file_path"]),
];

static OPTIONAL_VALUES: [&str; 1] = ["model_system_prompt"];

fn format_settings_key(key: String) -> String {
    key.split('_')
        .map(|word| {
            let mut chars = word.chars();
            match chars.next() {
                Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn format_settings_value(value: Value) -> String {
    let value = value.to_string();
    match value.as_str() {
        "true" => colorize_text(&"Enabled", AnsiColor::BrightGreen),
        "false" => colorize_text(&"Disabled", AnsiColor::BrightRed),
        "null" => colorize_text(&"Not set", AnsiColor::BrightBlack),
        _ if value.is_empty() => colorize_text(&"Not set", AnsiColor::BrightBlack),
        _ if value == "\"\"" => colorize_text(&"Not set", AnsiColor::BrightBlack),
        _ => value.to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsManager {
    pub model_name: String,
    pub show_thinking: bool,
    pub model_seed: i32,
    pub model_temperature: f32,
    pub model_system_prompt: String,
    pub verbose_tool_calls: bool,
    pub tool_confirmation: bool,
    pub auto_save_config: bool,
    pub config_file_path: PathBuf,
}

impl SettingsManager {
    pub async fn render_edit_menu(&mut self) {
        let json_value: Value =
            serde_json::to_value(self.clone()).expect("Failed to serialize settings");

        let mut choices = vec![];
        if let Value::Object(map) = json_value.clone() {
            for (key, value) in map {
                choices.push(MenuChoice {
                    name: format!(
                        "{}: {}",
                        format_settings_key(key),
                        format_settings_value(value)
                    ),
                    shortcut: '#',
                });
            }
        }

        let index = menu_selection("Choose settings to edit : ", choices, true).await;

        let key = json_value
            .as_object()
            .and_then(|obj| obj.keys().nth(index as usize))
            .cloned()
            .unwrap_or_default();

        let current_value = json_value.get(&key).cloned().unwrap_or(Value::Null);

        match current_value {
            Value::String(_) => {
                let new_value = text_input(&format!("New value for {}: ", key));
                if !new_value.is_empty() || OPTIONAL_VALUES.contains(&key.as_str()) {
                    self.update_setting(&key, Value::String(new_value));
                }
            }
            Value::Number(_) => {
                let new_value = text_input(&format!("New value for {}: ", key));
                if !new_value.is_empty() {
                    let new_value: Number = new_value.parse().unwrap_or(Number::from(0));
                    self.update_setting(&key, Value::Number(new_value));
                }
            }
            Value::Bool(_) => {
                let choices = vec![
                    MenuChoice {
                        name: "Enabled".to_string(),
                        shortcut: 'E',
                    },
                    MenuChoice {
                        name: "Disabled".to_string(),
                        shortcut: 'D',
                    },
                ];
                let choice = menu_selection(
                    &format!(
                        "Toggle {} (current: {})",
                        key,
                        if current_value.as_bool().unwrap_or(false) {
                            "Enabled"
                        } else {
                            "Disabled"
                        }
                    ),
                    choices,
                    true,
                )
                .await;
                self.update_setting(&key, Value::Bool(choice == 0));
            }
            _ => {
                println!("Unsupported setting type for {}", key);
            }
        }

        println!("Updated settings:\n{}", self.render(true));
    }

    fn update_setting(&mut self, key: &str, value: Value) {
        if let Ok(json_value) = serde_json::to_value(self.clone()).map(|v| {
            let mut obj = v.as_object().cloned().unwrap_or_default();
            obj.insert(key.to_string(), value);
            Value::Object(obj)
        }) {
            if let Ok(new_self) = serde_json::from_value::<SettingsManager>(json_value) {
                *self = new_self;
            }
        }

        if self.auto_save_config {
            self.save_to_file(&self.config_file_path)
                .expect("Failed to save updated settings to config file");
        }
    }

    pub fn load_from_file(file_path: &Path) -> AppResult<SettingsManager> {
        let content = read_to_string(file_path)?;
        let settings: SettingsManager = serde_json::from_str(&content)?;
        Ok(settings)
    }

    pub fn save_to_file(&self, file_path: &Path) -> AppResult<()> {
        let content = serde_json::to_string_pretty(self)?;
        write(file_path, content)?;
        Ok(())
    }
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self {
            model_name: "qwen2.5:7b".to_string(),
            show_thinking: true,
            model_seed: 0,
            model_temperature: 0.8,
            model_system_prompt: String::new(),
            tool_confirmation: true,
            config_file_path: get_config_path(ConfigFile::Settings),
            auto_save_config: true,
            verbose_tool_calls: true,
        }
    }
}

impl AppUIRenderable for SettingsManager {
    fn render(&self, boxed: bool) -> String {
        let json_value: Value = serde_json::to_value(self).expect("Failed to serialize settings");

        let mut formatted_content = String::new();
        if let Value::Object(map) = json_value {
            for (category, keys) in CATEGORIES.iter() {
                formatted_content.push_str(&colorize_text(
                    &format!("{}\n", category),
                    AnsiColor::BrightYellow,
                ));
                for key in *keys {
                    if let Some(value) = map.get(*key) {
                        formatted_content.push_str(&format!(
                            "{}: {}\n",
                            format_settings_key(key.to_string()),
                            format_settings_value(value.clone())
                        ));
                    }
                }

                if category != &CATEGORIES.last().unwrap().0 {
                    formatted_content.push_str("\n \n");
                }
            }
        } else {
            return "Invalid settings format".to_string();
        };

        if boxed {
            RoundedBox::new(
                &formatted_content,
                Some("Settings"),
                Some(AnsiColor::BrightYellow),
                false,
            )
            .render()
        } else {
            formatted_content
        }
    }
}
