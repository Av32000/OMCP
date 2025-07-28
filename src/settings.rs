use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ui::{AppUIRenderable, RoundedBox, utils::AnsiColor};

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
    println!("{}", value);
    let value = value.to_string();
    match value.as_str() {
        "true" => "Enabled".to_string(),
        "false" => "Disabled".to_string(),
        "null" => "Not set".to_string(),
        _ if value.is_empty() => "Not set".to_string(),
        _ => value.to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsManager {
    pub model_name: String,
    pub tool_confirmation: bool,
}

impl Default for SettingsManager {
    fn default() -> Self {
        Self {
            model_name: "qwen2.5:7b".to_string(),
            tool_confirmation: true,
        }
    }
}

impl AppUIRenderable for SettingsManager {
    fn render(&self, boxed: bool) -> String {
        let json_value: Value = serde_json::to_value(self).expect("Failed to serialize settings");

        let mut formatted_content = String::new();
        if let Value::Object(map) = json_value {
            for (key, value) in map {
                formatted_content.push_str(&format!(
                    "{}: {}\n",
                    format_settings_key(key),
                    format_settings_value(value)
                ));
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
