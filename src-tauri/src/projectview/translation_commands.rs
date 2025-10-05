use tauri::command;
use std::fs;
use std::path::Path;
use std::sync::Arc;
use serde_json::{Value, json};
use ort::{Environment, Session, SessionBuilder, Value as OrtValue, tensor::OrtOwnedTensor};
use rust_tokenizers::tokenizer::{SentencePieceTokenizer, Tokenizer, TruncationStrategy};
use rust_tokenizers::vocab::{Vocab};
use ndarray::{Array, s, CowArray, IxDyn};
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
    if let Some(node_text) = node.get("text").and_then(|v| v.as_str()) {
        text.push_str(node_text);
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
    let encoder_session = SessionBuilder::new(&environment)
        .map_err(|e| e.to_string())?
        .with_model_from_file(model_path.join("onnx/encoder_model.onnx"))
        .map_err(|e| e.to_string())?;
    let decoder_session = SessionBuilder::new(&environment)
        .map_err(|e| e.to_string())?
        .with_model_from_file(model_path.join("onnx/decoder_with_past_model.onnx"))
        .map_err(|e| e.to_string())?;

    let source_tokenizer = SentencePieceTokenizer::from_file(model_path.join("source.spm").to_str().unwrap(), false).map_err(|e| e.to_string())?;
    let target_tokenizer = SentencePieceTokenizer::from_file(model_path.join("target.spm").to_str().unwrap(), false).map_err(|e| e.to_string())?;

    let generation_config_content = fs::read_to_string(model_path.join("config.json")).map_err(|e| e.to_string())?;
    let generation_config: GenerationConfig = serde_json::from_str(&generation_config_content).map_err(|e| e.to_string())?;
    let decoder_start_token_id = generation_config.decoder_start_token_id;
    let eos_token_id = 0; // For MarianMT models, EOS is often the PAD token (0)

    let content = fs::read_to_string(&transcript_path).map_err(|e| e.to_string())?;
    let mut lexical_json: Value = serde_json::from_str(&content).map_err(|e| e.to_string())?;

    if let Some(table_node) = lexical_json.get_mut("root").and_then(|r| r.get_mut("children")).and_then(|c| c.as_array_mut()).and_then(|c| c.iter_mut().find(|n| n.get("type").and_then(|t| t.as_str()) == Some("table"))) {
        if let Some(rows) = table_node.get_mut("children").and_then(|c| c.as_array_mut()) {
            let texts_to_translate: Vec<String> = rows.iter().skip(1).filter_map(|row| {
                row.get("children").and_then(|c| c.as_array()).and_then(|cells| cells.get(3)).map(|cell| extract_plain_text_from_lexical(cell))
            }).collect();

            let mut translated_texts = Vec::new();
            for text in texts_to_translate {
                if text.trim().is_empty() {
                    translated_texts.push(String::new());
                    continue;
                }

                let translated_text = match translate_segment(&text, &encoder_session, &decoder_session, &source_tokenizer, &target_tokenizer, decoder_start_token_id, eos_token_id) {
                    Ok(translated) => translated,
                    Err(e) => {
                        error!("[Translate] Error translating segment: {}", e);
                        format!("[Translation Error: {}]", e)
                    }
                };
                translated_texts.push(translated_text);
            }

            for (row, translated_text) in rows.iter_mut().skip(1).zip(translated_texts.iter()) {
                if let Some(cells) = row.get_mut("children").and_then(|c| c.as_array_mut()) {
                    if cells.len() > 3 {
                        if let Some(text_cell) = cells.get_mut(3) {
                            let new_lexical_text = create_lexical_with_text(translated_text);
                            if let Some(new_children) = new_lexical_text.pointer("/root/children") {
                                text_cell["children"] = new_children.clone();
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

    save_transcript_json(project_xml_path, new_path.clone(), new_content, Some(target_lang))
        .await
        .map_err(|e| format!("[Translate] Failed to register translated transcript: {}", e))?;

    Ok(new_path)
}

fn translate_segment(
    text: &str,
    encoder: &Session,
    decoder: &Session,
    source_tokenizer: &SentencePieceTokenizer,
    target_tokenizer: &SentencePieceTokenizer,
    decoder_start_token_id: i64,
    eos_token_id: i64,
) -> Result<String, String> {
    // 1. Encoder Pass
    info!("[Translate] Encoding segment: '{}'", text);
    let tokenized_input = source_tokenizer.encode(text, None, 512, &TruncationStrategy::LongestFirst, 0);
    let input_ids: Vec<i64> = tokenized_input.token_ids.iter().map(|&x| x as i64).collect();
    let sequence_length = input_ids.len();

    let input_ids_array = Array::from_shape_vec((1, sequence_length), input_ids).map_err(|e| e.to_string())?;
    let attention_mask_array = Array::from_shape_vec((1, sequence_length), vec![1i64; sequence_length]).map_err(|e| e.to_string())?;

    let encoder_input_ids_cow = CowArray::from(input_ids_array.view()).into_dyn();
    let encoder_attention_mask_cow = CowArray::from(attention_mask_array.view()).into_dyn();
    let encoder_inputs = vec![
        OrtValue::from_array(encoder.allocator(), &encoder_input_ids_cow).map_err(|e| e.to_string())?,
        OrtValue::from_array(encoder.allocator(), &encoder_attention_mask_cow).map_err(|e| e.to_string())?,
    ];
    let encoder_outputs = encoder.run(encoder_inputs).map_err(|e| e.to_string())?;

    let encoder_hidden_state_tensor: OrtOwnedTensor<f32, IxDyn> = encoder_outputs[0].try_extract().map_err(|e| e.to_string())?;
    let encoder_hidden_state_array = encoder_hidden_state_tensor.view().to_owned();

    // 2. Decoder Loop
    info!("[Translate] Starting decoder loop.");
    let mut decoder_input_ids = Array::from_shape_vec((1, 1), vec![decoder_start_token_id]).map_err(|e| e.to_string())?;
    let mut generated_tokens: Vec<i64> = Vec::new();
    let mut past_key_values: Option<Vec<OrtOwnedTensor<f32, IxDyn>>> = None;
    let max_length = 128;

    for i in 0..max_length {
        let mut decoder_outputs = if let Some(past) = &past_key_values {
            info!("[Translate] Using past_key_values for loop iteration {}", i);
            let cow_decoder_input = CowArray::from(decoder_input_ids.view()).into_dyn();
            let mut inputs = vec![
                OrtValue::from_array(decoder.allocator(), &cow_decoder_input).map_err(|e| e.to_string())?,
            ];
            for past_tensor in past {
                inputs.push(OrtValue::from_array(decoder.allocator(), &CowArray::from(past_tensor.view()).into_dyn()).map_err(|e| e.to_string())?);
            }
            decoder.run(inputs).map_err(|e| e.to_string())?
        } else {
            info!("[Translate] First decoder loop iteration (no past_key_values)");
            let cow_decoder_input = CowArray::from(decoder_input_ids.view()).into_dyn();
            let cow_encoder_state = CowArray::from(encoder_hidden_state_array.view()).into_dyn();
            let inputs = vec![
                OrtValue::from_array(decoder.allocator(), &cow_decoder_input).map_err(|e| e.to_string())?,
                OrtValue::from_array(decoder.allocator(), &cow_encoder_state).map_err(|e| e.to_string())?,
            ];
            decoder.run(inputs).map_err(|e| e.to_string())?
        };

        let logits_value = decoder_outputs.remove(0);
        let logits: OrtOwnedTensor<f32, IxDyn> = logits_value.try_extract().map_err(|e| e.to_string())?;
        let next_token_logits = logits.view().slice(s![0, -1, ..]).to_owned();

        let next_token_id = next_token_logits
            .iter()
            .enumerate()
            .max_by(|(_, a), (_, b)| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal))
            .map(|(index, _)| index as i64)
            .unwrap_or(eos_token_id);

        if next_token_id == eos_token_id {
            info!("[Translate] End-of-sentence token found. Stopping generation.");
            break;
        }

        generated_tokens.push(next_token_id);
        decoder_input_ids = Array::from_shape_vec((1, 1), vec![next_token_id]).map_err(|e| e.to_string())?;

        let new_past: Result<Vec<OrtOwnedTensor<f32, IxDyn>>, String> = decoder_outputs
            .into_iter()
            .map(|value| value.try_extract().map_err(|e| e.to_string()))
            .collect();
        past_key_values = Some(new_past?);
    }

    // 3. Decode final output
    info!("[Translate] Decoding final tokens.");
    let translated_text = target_tokenizer.decode(&generated_tokens, true, true);
    info!("[Translate] Translated text: '{}'", translated_text);

    Ok(translated_text)
}