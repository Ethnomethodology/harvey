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

// Read and copy files
try {
    const files = fs.readdirSync(SOURCE_DIR);
    let copiedCount = 0;

    files.forEach(file => {
        if (path.extname(file) === '.md') {
            const srcPath = path.join(SOURCE_DIR, file);
            const destPath = path.join(DEST_DIR, file);
            fs.copyFileSync(srcPath, destPath);
            console.log(`Copied: ${file}`);
            copiedCount++;
        }
    });

    console.log(`Sync complete. Copied ${copiedCount} files.`);
} catch (error) {
    console.error(`Error syncing files:`, error);
    process.exit(1);
}

// --- Version Syncing Logic ---
console.log(`Syncing Application Version...`);
const TAURI_CONF_PATH = path.join(ROOT_DIR, 'src-tauri', 'tauri.conf.json');
const SVELTE_PAGE_PATH = path.join(ROOT_DIR, 'website', 'src', 'routes', '+page.svelte');

try {
    if (fs.existsSync(TAURI_CONF_PATH) && fs.existsSync(SVELTE_PAGE_PATH)) {
        const tauriConf = JSON.parse(fs.readFileSync(TAURI_CONF_PATH, 'utf8'));
        const version = tauriConf.version;
        console.log(`Current version in tauri.conf.json: ${version}`);

        let svelteContent = fs.readFileSync(SVELTE_PAGE_PATH, 'utf8');

        // Update version
        svelteContent = svelteContent.replace(
            /let version = ".*"; \/\* @sync-version \*\//,
            `let version = "${version}"; /* @sync-version */`
        );

        // Update links
        svelteContent = svelteContent.replace(
            /windows: ".*", \/\* @sync-win \*\//,
            `windows: "https://github.com/Ethnomethodology/harvey/releases/download/v${version}/Harvey_${version}_x64-setup.zip", /* @sync-win */`
        );
        svelteContent = svelteContent.replace(
            /macosArm: ".*", \/\* @sync-macos-arm \*\//,
            `macosArm: "https://github.com/Ethnomethodology/harvey/releases/download/v${version}/Harvey_${version}_aarch64.dmg", /* @sync-macos-arm */`
        );
        svelteContent = svelteContent.replace(
            /macosIntel: ".*", \/\* @sync-macos-x64 \*\//,
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
