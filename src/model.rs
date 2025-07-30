use ollama_rs::Ollama;

use crate::{
    AppResult,
    ui::{
        input::{self, MenuChoice},
        utils::{AnsiColor, colorize_text},
    },
};

pub async fn select_model(ollama: &Ollama) -> AppResult<String> {
    let models = ollama.list_local_models().await.unwrap_or_default();

    if models.is_empty() {
        return Err("No models available".into());
    }

    let mut choices = models
        .iter()
        .map(|model| MenuChoice {
            name: model.name.clone(),
            shortcut: '#',
        })
        .collect::<Vec<_>>();

    choices.push(MenuChoice {
        name: "Other".to_string(),
        shortcut: '#',
    });

    let selected_model = input::menu_selection("Select a model : ", choices.clone(), true).await;

    if selected_model as usize == choices.len() - 1 {
        let model_name = input::text_input("Enter model name: ");
        return Ok(model_name);
    }

    Ok(models[selected_model as usize].name.clone())
}

pub async fn render_model_info(model_name: String, ollama: &Ollama) -> String {
    let mut printed_info = String::new();

    match ollama.show_model_info(model_name.clone()).await {
        Ok(info) => {
            let mut parameters_string = String::new();

            if let Some(params) = info.model_info.get("general.parameter_count") {
                parameters_string.push_str(&params.to_string());
            }

            if let Some(params) = info.model_info.get("general.size_label") {
                if parameters_string.is_empty() {
                    parameters_string.push_str(&params.to_string());
                } else {
                    parameters_string.push_str(&format!(" ({})", params));
                }
            }

            if parameters_string.is_empty() {
                parameters_string = "Not available".to_string()
            };

            printed_info.push_str(&format!(
                "Model Name: {}\nParameters: {}\n",
                model_name, parameters_string
            ));
            printed_info.push_str("\n \n");
            printed_info.push_str(&colorize_text("Capabilities\n", AnsiColor::BrightBlue));
            printed_info.push_str(
                info.capabilities
                    .iter()
                    .map(|cap| format!("- {}\n", cap))
                    .collect::<String>()
                    .as_str(),
            );
        }
        Err(err) => {
            printed_info.push_str("Unable to retrieve model info");
        }
    };

    printed_info
}
