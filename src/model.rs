use ollama_rs::Ollama;

use crate::{
    AppResult,
    ui::input::{self, MenuChoice},
};

pub async fn select_model(ollama: Ollama) -> AppResult<String> {
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
