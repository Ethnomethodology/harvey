/**
 * Extracts a human-readable error message from a Tauri CommandError object.
 * The backend CommandError is serialized as { type: string, payload: any }.
 * 
 * @param {any} err The error object return by a Tauri invoke call.
 * @returns {string} A string representing the error message.
 */
export function getErrorMessage(err) {
  if (typeof err === 'string') return err;
  
  // Handle Tauri backend CommandError structure: { type: "Variant", payload: "Message" }
  if (err && typeof err === 'object') {
    if (err.payload && typeof err.payload === 'string') {
      return err.payload.replace(/^E_DIR_EXISTS:/, ''); // Strip prefix if present
    }
    if (err.message && typeof err.message === 'string') {
      return err.message;
    }
  }

  // Fallback for other error types or native JS errors
  try {
    return err?.message || JSON.stringify(err);
  } catch (e) {
    return 'An unknown error occurred';
  }
}

/**
 * Checks if a Tauri backend error is specifically a directory existence error.
 * 
 * @param {any} err The error object.
 * @returns {boolean} True if it's an E_DIR_EXISTS error.
 */
export function isDirExistsError(err) {
  if (!err || typeof err !== 'object') return false;
  const message = err.payload || err.message;
  return typeof message === 'string' && message.startsWith('E_DIR_EXISTS:');
}
