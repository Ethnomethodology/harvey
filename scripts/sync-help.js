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
