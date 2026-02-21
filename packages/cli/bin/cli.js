#!/usr/bin/env node

const { spawn } = require('child_process');
const path = require('path');
const fs = require('fs');
const os = require('os');

/**
 * Tunnel CLI NPM Wrapper
 * This script identifies the appropriate Rust binary for the current system
 * and executes it with passed arguments.
 */

function getBinaryPath() {
    const platform = os.platform();
    const arch = os.arch();

    // In a real production scenario, these would be downloaded on install
    // For this environment, we check local build path or a pre-defined location.

    let binName = platform === 'win32' ? 'xpose.exe' : 'xpose';

    // Check locally built binary first (for development/demo)
    const localBuild = path.join(__dirname, '..', 'target', 'release', binName);
    if (fs.existsSync(localBuild)) {
        return localBuild;
    }

    // Fallback path in user home (where our Rust downloader might place it)
    const homeDir = os.homedir();
    const installPath = path.join(homeDir, '.xpose', 'bin', binName);
    if (fs.existsSync(installPath)) {
        return installPath;
    }

    return null;
}

const binPath = getBinaryPath();

if (!binPath) {
    console.error('\x1b[31mError:\x1b[0m xpose binary not found.');
    console.log('\nPlease build the project using \x1b[33mcargo build --release\x1b[0m');
    console.log('Or visit the project repository to download a pre-built binary.');
    process.exit(1);
}

// Delegate everything to the Rust binary
const args = process.argv.slice(2);
const child = spawn(binPath, args, {
    stdio: 'inherit',
    env: {
        ...process.env,
        // Ensure color support is passed through if needed
        COLORTERM: 'truecolor'
    }
});

child.on('exit', (code) => {
    process.exit(code || 0);
});

child.on('error', (err) => {
    console.error('\x1b[31mFailed to start xpose:\x1b[0m', err.message);
    process.exit(1);
});
