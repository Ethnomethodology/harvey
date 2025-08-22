import { sep } from '@tauri-apps/api/path';

/**
 * Calculates the relative path of a file from a base directory.
 * @param {string} absolutePath The full, absolute path of the file.
 * @param {string} baseDirectory The full, absolute path of the directory to make the path relative to.
 * @returns {string|null} The relative path, or null if the file is not within the base directory.
 */
export function getRelativePath(absolutePath, baseDirectory) {
    if (!absolutePath || !baseDirectory) {
        return null;
    }

    // Ensure both paths use a consistent separator for comparison
    const normalizedAbsolutePath = absolutePath.replace(/\\/g, '/');
    const normalizedBaseDirectory = baseDirectory.replace(/\\/g, '/');

    if (normalizedAbsolutePath.startsWith(normalizedBaseDirectory)) {
        let relativePath = normalizedAbsolutePath.substring(normalizedBaseDirectory.length);
        if (relativePath.startsWith('/')) {
            relativePath = relativePath.substring(1);
        }
        return relativePath;
    }

    // Fallback for cases where path might not be normalized as expected, using sep
    if (absolutePath.startsWith(baseDirectory)) {
        let relativePath = absolutePath.substring(baseDirectory.length);
        if (relativePath.startsWith(sep)) {
            relativePath = relativePath.substring(sep.length);
        }
        return relativePath.replace(/\\/g, '/');
    }

    return null;
}
