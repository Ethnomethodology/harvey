use tauri::command;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use serde_json::{Value, json};
use ort::{Environment, SessionBuilder, Value as OrtValue, tensor::OrtOwnedTensor};
use rust_tokenizers::tokenizer::{SentencePieceBpeTokenizer, Tokenizer, TruncationStrategy};
use ndarray::{Array, Axis, CowArray};
use crate::welcome::config::{read_config, get_default_download_location};

fn extract_plain_text_from_lexical(node: &Value) -> String {
    let mut text = String::new();
    if let Some(node_type) = node.get("type").and_then(|v| v.as_str()) {
        if node_type == "text" || node_type == "extended-text" {
            if let Some(node_text) = node.get("text").and_then(|v| v.as_str()) {
                text.push_str(node_text);
            }
        }
    }

    if let Some(children) = node.get("children").and_then(|v| v.as_array()) {
        for child in children {
            text.push_str(&extract_plain_text_from_lexical(child));
        }
    }
    text
}

fn create_lexical_with_text(text: &str) -> Value {
    json!({
        "root": {
            "type": "root",
            "children": [{
                "type": "paragraph",
                "children": [{
                    "type": "text",
                    "text": text,
                    "detail": 0,
                    "format": 0,
                    "mode": "normal",
                    "style": "",
                    "version": 1
                }],
                "direction": "ltr",
                "format": "",
                "indent": 0,
                "version": 1
            }],
            "direction": "ltr",
            "format": "",
            "indent": 0,
            "version": 1
        }
    })
}

#[command]
pub async fn translate_transcript_command(transcript_path: String, source_lang: String, target_lang: String) -> Result<String, String> {
    let config = read_config().map_err(|e| e.to_string())?;
    let download_location = if !config.download_location.trim().is_empty() {
        config.download_location.clone()
    } else {
        get_default_download_location().map_err(|e| e.to_string())?
    };

    let model_name = format!("onnx-community/opus-mt-{}-{}", source_lang, target_lang);
    let model_path = Path::new(&download_location).join(&model_name);

    if !model_path.exists() {
        return Err(format!("Model '{}' not found at '{}'", model_name, model_path.display()));
    }

    let environment = Arc::new(Environment::builder().with_name("test").build().map_err(|e| e.to_string())?);
    let session = SessionBuilder::new(&environment)
        .and_then(|builder| builder.with_model_from_file(model_path.join("model.onnx")))
        .map_err(|e| e.to_string())?;

    let vocab_path = model_path.join("vocab.json");
    let tokenizer = SentencePieceBpeTokenizer::from_file(
        vocab_path.to_str().unwrap(),
        false,
    ).map_err(|e| e.to_string())?;

    let content = fs::read_to_string(&transcript_path).map_err(|e| e.to_string())?;
    let mut lexical_json: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    let table_node = lexical_json.get_mut("root")
        .and_then(|root| root.get_mut("children"))
        .and_then(|children| children.as_array_mut())
        .and_then(|children| children.iter_mut().find(|node| node.get("type").and_then(|t| t.as_str()) == Some("table")));

    if let Some(table_node) = table_node {
        if let Some(rows) = table_node.get_mut("children").and_then(|c| c.as_array_mut()) {
            let mut texts_to_translate = Vec::new();
            for row in rows.iter().skip(1) { // Skip header row
                if let Some(cells) = row.get("children").and_then(|c| c.as_array()) {
                    if cells.len() > 3 {
                        if let Some(text_cell) = cells.get(3) {
                            let lexical_segment: Value = json!({
                                "root": {
                                    "children": text_cell.get("children").unwrap_or(&Value::Array(vec![])).clone()
                                }
                            });
                            texts_to_translate.push(extract_plain_text_from_lexical(&lexical_segment));
                        }
                    }
                }
            }

            let texts_to_translate_str: Vec<&str> = texts_to_translate.iter().map(|s| s.as_str()).collect();
            let tokenized_input = tokenizer.encode_list(&texts_to_translate_str, 512, &TruncationStrategy::LongestFirst, 0);

            let max_len = tokenized_input.iter().map(|t| t.token_ids.len()).max().unwrap_or(0);
            let mut input_ids = Vec::new();
            for tokens in &tokenized_input {
                let mut token_ids = tokens.token_ids.clone();
                token_ids.resize(max_len, 0);
                input_ids.extend_from_slice(&token_ids);
            }

            let allocator = session.allocator();
            let array = Array::from_shape_vec((tokenized_input.len(), max_len), input_ids).unwrap().mapv(|x| x as i64);
            let cow_array = CowArray::from(array);
            let dyn_array = cow_array.into_dyn();
            let inputs: Vec<OrtValue> = vec![
                OrtValue::from_array(allocator, &dyn_array).unwrap(),
            ];

            let outputs: Vec<OrtValue> = session.run(inputs).map_err(|e| e.to_string())?;
            let output_tensor: OrtOwnedTensor<i64, _> = outputs[0].try_extract().unwrap();

            let mut translated_texts = Vec::new();
            for token_ids in output_tensor.view().axis_iter(Axis(0)) {
                let translated_text = tokenizer.decode(token_ids.as_slice().unwrap(), true, true);
                translated_texts.push(translated_text);
            }

            let mut translated_index = 0;
            for row in rows.iter_mut().skip(1) {
                if let Some(cells) = row.get_mut("children").and_then(|c| c.as_array_mut()) {
                    if cells.len() > 3 {
                        if let Some(text_cell) = cells.get_mut(3) {
                             if translated_index < translated_texts.len() {
                                let new_lexical_text = create_lexical_with_text(&translated_texts[translated_index]);
                                if let Some(new_children) = new_lexical_text.get("root").and_then(|r| r.get("children")).and_then(|c| c.as_array()) {
                                    text_cell["children"] = Value::Array(new_children.clone());
                                }
                                translated_index += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    let new_path = transcript_path.replace(".json", &format!(".{}.json", target_lang));
    let new_content = serde_json::to_string_pretty(&lexical_json).map_err(|e| e.to_string())?;
    fs::write(&new_path, new_content).map_err(|e| e.to_string())?;

    Ok(new_path)
}
