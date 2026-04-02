import path from 'path';
import { fileURLToPath } from 'url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);

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
            application: path.join(__dirname, 'src-tauri', 'target', 'debug', 'harvey')
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
