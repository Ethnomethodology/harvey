// src/lib/services/configureActions.js

import { invoke } from '@tauri-apps/api/core';
import { get } from 'svelte/store';
import { project } from '$lib/stores/projectStore.js';
import { transcriptStore } from '$lib/stores/transcriptStore.js';
import { $getRoot, $createParagraphNode, $createTextNode, $parseSerializedNode } from 'lexical';
import { createHeadlessEditor } from '@lexical/headless';
// --- REVISED IMPORT: Try importing writeTextFile specifically ---
import { writeTextFile } from '@tauri-apps/plugin-fs'; // For writing the file

// Import all nodes used in the transcript editor to ensure parsing works
import { ParagraphNode, RootNode, TextNode, LineBreakNode } from 'lexical';
import { HeadingNode, QuoteNode } from '@lexical/rich-text';
import { ListNode, ListItemNode } from '@lexical/list';
import { LinkNode } from '@lexical/link';
import { TableNode, TableRowNode, TableCellNode } from '@lexical/table';
import { ExtendedTextNode } from '$lib/nodes/ExtendedTextNode.js';

const ALL_EDITOR_NODES = [
	RootNode, ParagraphNode, TextNode, ExtendedTextNode, LineBreakNode,
	HeadingNode, QuoteNode, ListNode, ListItemNode, LinkNode,
	TableNode, TableRowNode, TableCellNode
];

// --- Existing functions (saveDownloadLocation, etc.) ... ---
export async function saveDownloadLocation(downloadLocation) {
  try {
	await invoke('save_download_location', { newLocation: downloadLocation });
	console.log("Download location direct update sent to backend:", downloadLocation);
	return true;
  } catch (error) {
	console.error("Error invoking save_download_location:", error);
	throw new Error(`Failed to save download location directly: ${error?.message || error}`);
  }
}
export async function getDownloadLocation() {
  try {
	const location = await invoke('get_download_location');
	console.log("Retrieved download location from backend:", location);
	return location || "";
  } catch (error) {
	console.error("Error invoking get_download_location:", error);
	return "";
  }
}
export async function getDownloadedModels() {
  try {
	const models = await invoke('get_downloaded_models');
	console.log("Retrieved downloaded models from backend:", models);
	// Filter out translation models (those with a family set or containing '/')
	const transcriptionModels = Array.isArray(models) ? models.filter(model => !model.family && !model.name.includes('/')) : [];
	return transcriptionModels;
  } catch (error) {
	console.error("Error invoking get_downloaded_models:", error);
	return [];
  }
}
export async function getAllDownloadedModels() {
  try {
	const models = await invoke('get_downloaded_models');
	return Array.isArray(models) ? models : [];
  } catch (error) {
	console.error("Error invoking get_downloaded_models:", error);
	return [];
  }
}
export async function downloadModel(model, downloadLocation) {
  if (!model?.download_url) {
	  const errorMsg = `Model "${model?.name || 'Unknown'}" is missing a download URL.`;
	  console.error(errorMsg);
	  throw new Error(errorMsg);
  }
  if (!downloadLocation || downloadLocation.trim() === '') {
	   const errorMsg = `Download location is not set. Cannot download model.`;
	   console.error(errorMsg);
	   throw new Error(errorMsg);
  }
  console.log(`Attempting to download model: ${model.name} from ${model.download_url} to ${downloadLocation}`);
  try {
	await invoke('download_model_command', {
	  modelInfo: model,
	  downloadLocation: downloadLocation
	});
	console.log(`Download command invoked for model: ${model.name}`);
	return true; // Signifies invocation success
  } catch (error) {
	console.error(`Error invoking download_model_command for ${model.name}:`, error);
	throw new Error(`Failed to start model download: ${error?.message || error}`);
  }
}
export async function deleteModel(model) {
  console.log(`Attempting to delete model: ${model.name}`);
  try {
	 if (!model?.name) {
		 const errorMsg = `Cannot delete model without a name.`;
		 console.error(errorMsg);
		 throw new Error(errorMsg);
	 }
	await invoke('delete_model', { modelToDelete: model });
	console.log("Model deletion command invoked:", model.name);
	return true;
  } catch (error) {
	console.error(`Error invoking delete_model for ${model.name}:`, error);
	throw new Error(`Failed to delete model: ${error?.message || error}`);
  }
}
export async function cancelDownload(modelName) {
	if (!modelName) {
		console.error("Cannot cancel download without a model name.");
		return;
	}
	console.log(`Requesting cancellation for model: ${modelName}`);
	try {
		await invoke('cancel_download_command', { modelName: modelName });
		console.log(`Cancellation command invoked for ${modelName}.`);
	} catch (error) {
		console.error(`Error invoking cancel_download_command for ${modelName}:`, error);
		throw new Error(`Failed to request download cancellation: ${error?.message || error}`);
	}
}
export async function moveModelsAndUpdateLocation(newLocation) {
	if (!newLocation || newLocation.trim() === '') {
		 const errorMsg = `New download location cannot be empty.`;
		 console.error(errorMsg);
		 throw new Error(errorMsg);
	}
	console.log(`Attempting to change download location and move models to: ${newLocation}`);
	try {
		await invoke('change_download_location_and_move_models', { newLocation: newLocation });
		console.log(`Backend successfully moved models (if any) and updated location to: ${newLocation}`);
		return true; // Indicate success
	} catch (error) {
		console.error(`Error invoking change_download_location_and_move_models to ${newLocation}:`, error);
		throw new Error(`Failed to move models/update location: ${error?.message || error}`);
	}
}

// --- Translation Model Actions ---
export async function downloadTranslationModel(from, to, downloadLocation, modelName = null, family = 'helsinki') {
  let to_lang = to;
  if (to === 'ja') {
    to_lang = 'jap';
  }
  const model_name = modelName || `Helsinki-NLP/opus-mt-${from}-${to_lang}`;
  const modelInfo = {
    name: model_name,
    family: family,
    download_url: `https://huggingface.co/${model_name}`
  };
  if (!downloadLocation || downloadLocation.trim() === '') {
    const errorMsg = `Download location is not set. Cannot download translation model.`;
    console.error(errorMsg);
    throw new Error(errorMsg);
  }
  console.log(`Attempting to download translation model: ${model_name} (family: ${family}) to ${downloadLocation}`);
  try {
    await invoke('download_translation_model_command', {
      modelInfo: modelInfo,
      downloadLocation: downloadLocation
    });
    console.log(`Download command invoked for translation model: ${model_name}`);
    return true;
  } catch (error) {
    console.error(`Error invoking download_translation_model_command for ${model_name}:`, error);
    throw new Error(`Failed to start translation model download: ${error?.message || error}`);
  }
}

export async function getSelectedTranslationFamily() {
	try {
		return await invoke('get_selected_translation_family');
	} catch (error) {
		console.error("Error invoking get_selected_translation_family:", error);
		return 'helsinki';
	}
}

export async function setSelectedTranslationFamily(family) {
	try {
		await invoke('set_selected_translation_family', { family });
		return true;
	} catch (error) {
		console.error("Error invoking set_selected_translation_family:", error);
		return false;
	}
}

export async function deleteTranslationModel(model) {
  console.log(`Attempting to delete translation model: ${model.name}`);
  try {
    if (!model?.name) {
      const errorMsg = `Cannot delete translation model without a name.`;
      console.error(errorMsg);
      throw new Error(errorMsg);
    }
    // This uses the same backend `delete_model` command, which is fine as it just deletes the folder.
    await invoke('delete_model', { modelToDelete: model });
    console.log("Translation model deletion command invoked:", model.name);
    return true;
  } catch (error) {
    console.error(`Error invoking delete_model for translation model ${model.name}:`, error);
    throw new Error(`Failed to delete translation model: ${error?.message || error}`);
  }
}

export async function cancelTranslationModelDownload(modelName) {
	if (!modelName) {
		console.error("Cannot cancel download without a model name.");
		return;
	}
	console.log(`Requesting cancellation for translation model: ${modelName}`);
	try {
        // This uses the same backend `cancel_download_command`, which is fine as it works by model name.
		await invoke('cancel_download_command', { modelName: modelName });
		console.log(`Cancellation command invoked for ${modelName}.`);
	} catch (error) {
		console.error(`Error invoking cancel_download_command for ${modelName}:`, error);
		throw new Error(`Failed to request download cancellation: ${error?.message || error}`);
	}
}

export async function getLocalTranslationModels() {
	try {
		const models = await invoke('get_local_translation_models');
		console.log("Retrieved local translation models from backend:", models);
		return Array.isArray(models) ? models : [];
	} catch (error) {
		console.error("Error invoking get_local_translation_models:", error);
		return [];
	}
}

export async function fetchAvailableModels() {
	try {
		const models = await invoke('fetch_available_models_command');
		console.log("Fetched available models from backend:", models ? models.length : 0);
		return Array.isArray(models) ? models : [];
	} catch (error) {
		console.error("Error invoking fetch_available_models_command:", error);
		throw new Error(`Failed to fetch available models: ${error?.message || error}`);
	}
}

// --- Export Action ---
/**
 * Exports the current transcript segments to a specified file path and format.
 * Currently only supports 'csv' format by generating CSV on the frontend.
 * @param {string} filePath - The full path to save the exported file.
 * @param {string} format - The desired export format ('csv', 'docx').
 * @param {Array<object>} segments - The array of segment data to export.
 * @param {string} transcriptJsonPath - The path to the transcript JSON file (used for DOCX export).
 * @param {string} [layoutChoice] - Optional. The chosen layout for DOCX export (e.g., 'Layout1', 'Layout2').
 * @returns {Promise<void>} A promise that resolves when export is complete or rejects on error.
 */
export async function exportTranscript(filePath, format, segments, transcriptJsonPath, layoutChoice) {
	console.log(`[ConfigureActions] Attempting export to "${filePath}" (Format: "${format}", Layout: "${layoutChoice || 'default'}")`);

	if (format !== 'docx' && (!segments || segments.length === 0)) { // Segments not needed upfront for docx if using transcriptJsonPath
		throw new Error("No transcript segments available to export.");
	}

    const { activeTranscript } = get(transcriptStore);
    if (activeTranscript && activeTranscript.path === transcriptJsonPath) {
        segments = activeTranscript.segments;
    }
	if (!filePath || filePath.trim() === '') {
		throw new Error("Export file path is missing.");
	}

    // Handle DOCX export via backend
    if (format === 'docx') {
      if (!transcriptJsonPath) {
        throw new Error('Transcript JSON path is not set.');
      }
      try {
        const payload = {
          transcriptJsonPathStr: transcriptJsonPath,
          outputPathStr: filePath,
          layoutChoice: layoutChoice || 'Layout2' // Default to Layout2 if not provided
        };
        console.log('[ConfigureActions] Invoking export_transcript_to_docx with payload:', payload);
        const savedPath = await invoke('export_transcript_to_docx', payload);
        console.log(`[ConfigureActions] DOCX export successful: ${savedPath}`);
        return; // done
      } catch (err) {
        console.error('[ConfigureActions] Error during DOCX export:', err);
        throw new Error(`Failed to export DOCX: ${err?.message || err}`);
      }
    } else if (format === 'srt') {
      if (!segments || segments.length === 0) {
        throw new Error("No transcript segments available to export for SRT.");
      }
      try {
        const payload = {
          outputPathStr: filePath,
          segmentsJsonStr: JSON.stringify(segments) // Pass segments as JSON string
        };
        console.log('[ConfigureActions] Invoking export_transcript_to_srt with payload:', payload);
        const savedPath = await invoke('export_transcript_to_srt', payload);
        console.log(`[ConfigureActions] SRT export successful: ${savedPath}`);
        return; // done
      } catch (err) {
        console.error('[ConfigureActions] Error during SRT export:', err);
        throw new Error(`Failed to export SRT: ${err?.message || err}`);
      }
    } else if (format === 'vtt') {
      if (!segments || segments.length === 0) {
        throw new Error("No transcript segments available to export for VTT.");
      }
      try {
        const payload = {
          outputPathStr: filePath,
          segmentsJsonStr: JSON.stringify(segments) // Pass segments as JSON string
        };
        console.log('[ConfigureActions] Invoking export_transcript_to_vtt with payload:', payload);
        const savedPath = await invoke('export_transcript_to_vtt', payload);
        console.log(`[ConfigureActions] VTT export successful: ${savedPath}`);
        return; // done
      } catch (err) {
        console.error('[ConfigureActions] Error during VTT export:', err);
        throw new Error(`Failed to export VTT: ${err?.message || err}`);
      }
    } else if (format === 'md') {
      if (!segments || segments.length === 0) { // Markdown also needs segments
        throw new Error("No transcript segments available to export for Markdown.");
      }
      try {
        const payload = {
          outputPathStr: filePath,
          segmentsJsonStr: JSON.stringify(segments), // Pass segments as JSON string
          layoutChoice: layoutChoice || 'Layout2' // Default to Layout2 if not provided for MD
        };
        console.log('[ConfigureActions] Invoking export_transcript_to_markdown with payload:', payload);
        const savedPath = await invoke('export_transcript_to_markdown', payload);
        console.log(`[ConfigureActions] Markdown export successful: ${savedPath}`);
        return; // done
      } catch (err) {
        console.error('[ConfigureActions] Error during Markdown export:', err);
        throw new Error(`Failed to export Markdown: ${err?.message || err}`);
      }
    } else if (format === 'ass') {
      if (!segments || segments.length === 0) {
        throw new Error("No transcript segments available to export for ASS.");
      }
      try {
        const payload = {
          outputPathStr: filePath,
          segmentsJsonStr: JSON.stringify(segments) // Pass segments as JSON string
          // No layoutChoice needed for ASS
        };
        console.log('[ConfigureActions] Invoking export_transcript_to_ass with payload:', payload);
        const savedPath = await invoke('export_transcript_to_ass', payload);
        console.log(`[ConfigureActions] ASS export successful: ${savedPath}`);
        return; // done
      } catch (err) {
        console.error('[ConfigureActions] Error during ASS export:', err);
        throw new Error(`Failed to export ASS: ${err?.message || err}`);
      }
    } else if (format === 'csv') {
		// --- Frontend CSV Generation ---
		try {
			const csvRows = [];
			// CSV Header
			csvRows.push('"StartTime","EndTime","Speaker","Text"'); // Use quotes for safety

			// Create a single headless editor instance for text conversion
		const textConversionEditor = createHeadlessEditor({
			nodes: ALL_EDITOR_NODES,
			namespace: `csv-export-converter-${Math.random()}`,
			onError: (e) => console.error(`[CSV Export Converter] Error:`, e)
		});

		// Helper function to format time (e.g., 00:01:23.456)
		const formatTimestamp = (seconds) => {
			if (typeof seconds !== 'number' || isNaN(seconds) || seconds < 0) return '00:00:00.000';
			const totalMs = Math.round(seconds * 1000);
			const ms = String(totalMs % 1000).padStart(3, '0');
			const totalS = Math.floor(totalMs / 1000);
			const sec = String(totalS % 60).padStart(2, '0');
			const totalMin = Math.floor(totalS / 60);
			const min = String(totalMin % 60).padStart(2, '0');
			const hr = String(Math.floor(totalMin / 60)).padStart(2, '0');
			return `${hr}:${min}:${sec}.${ms}`;
		};

		// Helper function to safely escape CSV fields
		const escapeCsvField = (field) => {
			const strField = String(field ?? ''); // Ensure it's a string, handle null/undefined
			// If field contains comma, newline, or double quote, enclose in double quotes
			if (strField.includes(',') || strField.includes('\n') || strField.includes('"')) {
				// Escape existing double quotes by doubling them up
				const escaped = strField.replace(/"/g, '""');
				return `"${escaped}"`;
			}
			return strField; // Return as is if no special characters
		};

		// Helper to check if a string is valid JSON
		const isValidJson = (str) => {
			if (typeof str !== 'string' || !str.trim().startsWith('{') || !str.trim().endsWith('}')) {
				 return false;
			}
			try { JSON.parse(str); return true; }
			catch (e) { return false; }
		}

		// Helper to get plain text from Lexical JSON or return string if not JSON
		const getPlainText = (textData) => {
			if (!textData || typeof textData !== 'string') return '';

			if (isValidJson(textData)) {
				try {
					const parsedJson = JSON.parse(textData);
					// Check if it's a valid Lexical state structure
					if (parsedJson && parsedJson.root && Array.isArray(parsedJson.root.children)) {
						textConversionEditor.setEditorState(textConversionEditor.parseEditorState(parsedJson));
						return textConversionEditor.getEditorState().read(() => $getRoot().getTextContent());
					} else {
						console.warn('[CSV Export] Invalid Lexical JSON structure detected:', textData.substring(0, 100));
						return '[Invalid JSON Structure]'; // Indicate error in output
					}
				} catch (e) {
					console.warn('[CSV Export] Failed to parse JSON, returning original string:', e, textData.substring(0, 100));
					return textData; // Return the original string if parsing fails
				}
			} else {
				 // If it's not JSON, return the string directly
				 return textData;
			}
		};

		// Process each segment
		for (const segment of segments) {
			const startTime = formatTimestamp(segment.start_time);
			const endTime = formatTimestamp(segment.end_time);
			const speaker = segment.speaker || 'Unknown';
			const textData = segment.text || ''; // Can be JSON string or plain text

			const plainText = getPlainText(textData); // Use the robust helper

			const row = [
				escapeCsvField(startTime),
				escapeCsvField(endTime),
				escapeCsvField(speaker),
				escapeCsvField(plainText) // Escape the extracted/original plain text
			].join(',');
			csvRows.push(row);
		}

		const csvContent = csvRows.join('\n');

		// --- Attempting writeTextFile ---
		console.log(`[ConfigureActions] Writing ${csvContent.length} bytes of CSV content to ${filePath} using writeTextFile`);
        // Pass path and contents directly as arguments
		await writeTextFile(filePath, csvContent);
		// --- END ---

		

	} catch (error) {
		console.error(`[ConfigureActions] Error during CSV export generation or file writing:`, error);
		// Check if the error is because writeTextFile doesn't exist
        if (error instanceof TypeError && error.message.includes('writeTextFile is not a function')) {
             console.error("[ConfigureActions] writeTextFile function not found in fs plugin. Trying generic invoke...");
             // Fallback to generic invoke as a last resort (less type-safe)
             try {
                // Manually construct the invoke call structure
                 await invoke('plugin:fs|write_file', { // Assuming the backend command name follows this pattern
                     path: filePath,
                     contents: csvContent.split('').map(c => c.charCodeAt(0)) // Convert string to byte array (Uint8Array) manually
                 });
                  console.log(`[ConfigureActions] CSV Export successful to ${filePath} via generic invoke.`);
             } catch (invokeError) {
                 console.error(`[ConfigureActions] Generic invoke failed as well:`, invokeError);
                 throw new Error(`Failed to export CSV (invoke failed): ${invokeError?.message || invokeError}`);
             }
        } else {
            // Rethrow original error if it wasn't a TypeError about writeTextFile
		    throw new Error(`Failed to export CSV: ${error?.message || error}`);
        }
	}
} else {
	throw new Error(`Export format "${format}" is not supported.`);
}
}
// Reminder: When calling exportTranscript in TopBar.svelte, pass the transcript JSON path as the fourth argument.