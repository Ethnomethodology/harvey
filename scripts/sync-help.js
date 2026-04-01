import fs from 'fs';
import path from 'path';
import { fileURLToPath } from 'url';

// Resolving __dirname in ES modules
const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

// Paths relative to script location:
//   scripts/sync-help.js
//   website/src/content/help
//   src/content/help
const ROOT_DIR = path.resolve(__dirname, '..');
const SOURCE_DIR = path.join(ROOT_DIR, 'website', 'src', 'content', 'help');
const DEST_DIR = path.join(ROOT_DIR, 'src', 'content', 'help');

console.log(`Syncing Help Content...`);
console.log(`Source: ${SOURCE_DIR}`);
console.log(`Dest:   ${DEST_DIR}`);

if (!fs.existsSync(SOURCE_DIR)) {
    console.error(`Source directory not found: ${SOURCE_DIR}`);
    process.exit(1);
}

if (!fs.existsSync(DEST_DIR)) {
    console.log(`Creating destination directory...`);
    fs.mkdirSync(DEST_DIR, { recursive: true });
}

// --- Version Extraction ---
let currentVersion = '0.0.0';
const TAURI_CONF_PATH = path.join(ROOT_DIR, 'src-tauri', 'tauri.conf.json');
try {
    if (fs.existsSync(TAURI_CONF_PATH)) {
        const tauriConf = JSON.parse(fs.readFileSync(TAURI_CONF_PATH, 'utf8'));
        currentVersion = tauriConf.version;
        console.log(`Current version for sync: ${currentVersion}`);
    }
} catch (e) {
    console.warn("Could not read tauri.conf.json for version syncing.");
}

// Read and copy files
try {
    const files = fs.readdirSync(SOURCE_DIR);
    let copiedCount = 0;

    files.forEach(file => {
        if (path.extname(file) === '.md') {
            const srcPath = path.join(SOURCE_DIR, file);
            const destPath = path.join(DEST_DIR, file);
            
            let content = fs.readFileSync(srcPath, 'utf8');
            
            // Hardcode GitHub release links to the current version
            // This replaces /releases/latest or /releases/tag/vX.Y.Z with the current version from tauri.conf.json
            content = content.replace(
                /https:\/\/github\.com\/Ethnomethodology\/harvey\/releases\/(latest|tag\/v[0-9.]+)/g,
                `https://github.com/Ethnomethodology/harvey/releases/tag/v${currentVersion}`
            );

            fs.writeFileSync(destPath, content);
            fs.writeFileSync(srcPath, content); // Also update the website's source content
            console.log(`Synced & Hardcoded (Source & Dest): ${file}`);
            copiedCount++;
        }
    });

    console.log(`Sync complete. Copied and processed ${copiedCount} files.`);
} catch (error) {
    console.error(`Error syncing files:`, error);
    process.exit(1);
}

// --- Website Version Syncing Logic ---
console.log(`Syncing Application Version in Website...`);
const SVELTE_PAGE_PATH = path.join(ROOT_DIR, 'website', 'src', 'routes', '+page.svelte');

try {
    if (fs.existsSync(TAURI_CONF_PATH) && fs.existsSync(SVELTE_PAGE_PATH)) {
        const version = currentVersion;

        let svelteContent = fs.readFileSync(SVELTE_PAGE_PATH, 'utf8');

        // Update version
        svelteContent = svelteContent.replace(
            /let version = ".*"; \/\* @sync-version \*\//g,
            `let version = "${version}"; /* @sync-version */`
        );

        // Update links - making comma optional and using global flag
        svelteContent = svelteContent.replace(
            /windows: ".*"(,|) \/\* @sync-win \*\//g,
            `windows: "https://github.com/Ethnomethodology/harvey/releases/download/v${version}/Harvey_${version}_x64-setup.zip", /* @sync-win */`
        );
        svelteContent = svelteContent.replace(
            /macosArm: ".*"(,|) \/\* @sync-macos-arm \*\//g,
            `macosArm: "https://github.com/Ethnomethodology/harvey/releases/download/v${version}/Harvey_${version}_aarch64.dmg", /* @sync-macos-arm */`
        );
        svelteContent = svelteContent.replace(
            /macosIntel: ".*"(,|) \/\* @sync-macos-x64 \*\//g,
            `macosIntel: "https://github.com/Ethnomethodology/harvey/releases/download/v${version}/Harvey_${version}_x64.dmg", /* @sync-macos-x64 */`
        );

        fs.writeFileSync(SVELTE_PAGE_PATH, svelteContent);
        console.log(`Successfully updated version and links in website.`);
    } else {
        console.warn(`Skipping version sync: tauri.conf.json or +page.svelte not found.`);
    }
} catch (error) {
    console.error(`Error syncing version:`, error);
    // We don't exit(1) here to avoid breaking the help sync if version sync fails
}
