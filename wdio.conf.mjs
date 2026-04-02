import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

const isWin = process.platform === 'win32';
const appName = isWin ? 'harvey.exe' : 'harvey';
const binaryPath = path.join(__dirname, 'src-tauri', 'target', 'debug', appName);

export const config = {
    // 4444 is the default port for tauri-wd
    port: 4444,
    runner: 'local',
    specs: [
        './e2e-tests/**/*.e2e.js'
    ],
    exclude: [],
    maxInstances: 1,
    capabilities: [{
        browserName: 'chrome', // webdriverio needs a browsername even if unused
        'tauri:options': {
            application: binaryPath
        }
    }],
    logLevel: 'info',
    bail: 0,
    baseUrl: 'http://localhost',
    waitforTimeout: 10000,
    connectionRetryTimeout: 120000,
    connectionRetryCount: 3,
    framework: 'mocha',
    reporters: ['spec'],
    mochaOpts: {
        ui: 'bdd',
        timeout: 120000 // 2 minutes
    },
}
