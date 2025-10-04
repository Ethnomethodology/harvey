use tauri::command;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use serde_json::{Value, json};
use ort::{Environment, SessionBuilder, Value as OrtValue, tensor::OrtOwnedTensor};
use rust_tokenizers::tokenizer::{SentencePieceTokenizer, Tokenizer, TruncationStrategy};
use rust_tokenizers::vocab::{Vocab};
use ndarray::{Array, CowArray, s};
use crate::welcome::config::{read_config, get_default_download_location};
use log::{info, error};
use super::transcription_commands::save_transcript_json;
use serde::Deserialize;

#[derive(Deserialize)]
struct GenerationConfig {
    decoder_start_token_id: i64,
}

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
pub async fn translate_transcript_command(
    project_xml_path: String,
    transcript_path: String,
    source_lang: String,
    target_lang: String
) -> Result<String, String> {
    info!("[Translate] Starting translation for transcript: {}", transcript_path);
    info!("[Translate] Project XML path: {}", project_xml_path);
    info!("[Translate] Source language: {}, Target language: {}", source_lang, target_lang);

    let config = read_config().map_err(|e| e.to_string())?;
    let download_location = if !config.download_location.trim().is_empty() {
        config.download_location.clone()
    } else {
        get_default_download_location().map_err(|e| e.to_string())?
    };
    info!("[Translate] Using download location: {}", download_location);

    let model_name = format!("onnx-community/opus-mt-{}-{}", source_lang, target_lang);
    let model_path = Path::new(&download_location).join(&model_name);
    info!("[Translate] Model path: {}", model_path.display());

    if !model_path.exists() {
        let err_msg = format!("Model '{}' not found at '{}'", model_name, model_path.display());
        error!("[Translate] {}", err_msg);
        return Err(err_msg);
    }

    let environment = Arc::new(Environment::builder().with_name("test").build().map_err(|e| e.to_string())?);

    let encoder_model_path = model_path.join("encoder_model.onnx");
    info!("[Translate] Loading encoder model from: {}", encoder_model_path.display());
    let encoder_session = SessionBuilder::new(&environment)
        .and_then(|builder| builder.with_model_from_file(&encoder_model_path))
        .map_err(|e| e.to_string())?;

    let decoder_model_path = model_path.join("decoder_with_past_model.onnx");
    info!("[Translate] Loading decoder model from: {}", decoder_model_path.display());
    let decoder_session = SessionBuilder::new(&environment)
        .and_then(|builder| builder.with_model_from_file(&decoder_model_path))
        .map_err(|e| e.to_string())?;

    let source_vocab_path = model_path.join("source.spm");
    let target_vocab_path = model_path.join("target.spm");

    info!("[Translate] Loading source tokenizer from: {}", source_vocab_path.display());
    info!("[Translate] Loading target tokenizer from: {}", target_vocab_path.display());

    let source_tokenizer = SentencePieceTokenizer::from_file(source_vocab_path.to_str().unwrap(), false).map_err(|e| e.to_string())?;
    let target_tokenizer = SentencePieceTokenizer::from_file(target_vocab_path.to_str().unwrap(), false).map_err(|e| e.to_string())?;
    let target_vocab = target_tokenizer.vocab();

    let generation_config_path = model_path.join("generation_config.json");
    info!("[Translate] Loading generation config from: {}", generation_config_path.display());
    let generation_config_content = fs::read_to_string(generation_config_path).map_err(|e| e.to_string())?;
    let generation_config: GenerationConfig = serde_json::from_str(&generation_config_content).map_err(|e| e.to_string())?;
    let decoder_start_token_id = generation_config.decoder_start_token_id;
    info!("[Translate] Decoder start token ID: {}", decoder_start_token_id);

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
            info!("[Translate] Found {} text segments to translate.", texts_to_translate.len());

            let mut translated_texts = Vec::new();

            for (i, text) in texts_to_translate.iter().enumerate() {
                if text.trim().is_empty() {
                    info!("[Translate] Segment {} is empty, skipping translation.", i + 1);
                    translated_texts.push(String::new());
                    continue;
                }

                info!("[Translate] Translating segment {}: '{}'", i + 1, text);
                let tokenized_input = source_tokenizer.encode(text, None, 512, &TruncationStrategy::LongestFirst, 0);
                let input_ids = tokenized_input.token_ids.iter().map(|&x| x as i64).collect::<Vec<i64>>();
                let array = Array::from_shape_vec((1, input_ids.len()), input_ids).unwrap();
                let input_tensor = CowArray::from(array).into_dyn();
                let inputs = vec![OrtValue::from_array(encoder_session.allocator(), &input_tensor).unwrap()];

                let encoder_outputs: Vec<OrtValue> = encoder_session.run(inputs).map_err(|e| e.to_string())?;
                let encoder_hidden_states: OrtOwnedTensor<f32, _> = encoder_outputs[0].try_extract().unwrap();

                let mut decoder_input_ids = vec![decoder_start_token_id];
                let mut translated_tokens = Vec::new();

                for _ in 0..512 { // Max length
                    let decoder_input_array = Array::from_shape_vec((1, decoder_input_ids.len()), decoder_input_ids.clone()).unwrap();
                    let owned_encoder_states = encoder_hidden_states.view().to_owned();

                    let decoder_input_tensor = CowArray::from(decoder_input_array).into_dyn();
                    let encoder_states_tensor = CowArray::from(owned_encoder_states).into_dyn();

                    let decoder_inputs = vec![
                        OrtValue::from_array(decoder_session.allocator(), &decoder_input_tensor).unwrap(),
                        OrtValue::from_array(decoder_session.allocator(), &encoder_states_tensor).unwrap(),
                    ];

                    let decoder_outputs: Vec<OrtValue> = decoder_session.run(decoder_inputs).map_err(|e| e.to_string())?;
                    let logits: OrtOwnedTensor<f32, _> = decoder_outputs[0].try_extract().unwrap();
                    let logits_view = logits.view();
                    let last_token_logits = logits_view.slice(s![0, -1, ..]);

                    let next_token_id = last_token_logits
                        .iter()
                        .enumerate()
                        .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
                        .map(|(index, _)| index)
                        .unwrap_or(0) as i64;

                    if next_token_id == target_vocab.token_to_id("</s>") {
                        break;
                    }

                    translated_tokens.push(next_token_id);
                    decoder_input_ids.push(next_token_id);
                }

                let translated_text = target_tokenizer.decode(&translated_tokens, true, true);
                info!("[Translate] Translated segment {}: '{}'", i + 1, translated_text);
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
    fs::write(&new_path, &new_content).map_err(|e| e.to_string())?;
    info!("[Translate] Translation file saved to: {}", new_path);

    info!("[Translate] Registering new transcript in project metadata...");
    save_transcript_json(
        project_xml_path,
        new_path.clone(),
        new_content,
        Some(target_lang),
    ).await.map_err(|e| {
        let err_msg = format!("[Translate] Failed to register translated transcript in project XML: {}", e);
        error!("{}", err_msg);
        err_msg
    })?;

    info!("[Translate] Translation and registration complete for: {}", new_path);
    Ok(new_path)
}