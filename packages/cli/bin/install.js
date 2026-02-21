#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');
const https = require('https');

/**
 * xpose binary downloader
 * Fetches the pre-compiled Rust binary for the current platform.
 */

const VERSION = '0.1.0';
const REPO = 'user/xpose-cli'; // Placeholder
const BASE_URL = `https://github.com/${REPO}/releases/download/v${VERSION}`;

const BIN_DIR = path.join(os.homedir(), '.xpose', 'bin');

function getReleaseName() {
    const platform = os.platform();
    const arch = os.arch();

    const targets = {
        'linux-x64': 'x86_64-unknown-linux-musl',
        'darwin-x64': 'x86_64-apple-darwin',
        'darwin-arm64': 'aarch64-apple-darwin',
        'win32-x64': 'x86_64-pc-windows-msvc'
    };

    const targetKey = `${platform}-${arch}`;
    const target = targets[targetKey];

    if (!target) {
        console.error(`Unsupported platform/architecture: ${targetKey}`);
        process.exit(1);
    }

    return `xpose-${target}.tar.gz`;
}

async function download() {
    if (!fs.existsSync(BIN_DIR)) {
        fs.mkdirSync(BIN_DIR, { recursive: true });
    }

    const releaseName = getReleaseName();
    const url = `${BASE_URL}/${releaseName}`;
    const dest = path.join(BIN_DIR, releaseName);

    console.log(`Downloading xpose binary from ${url}...`);

    const file = fs.createWriteStream(dest);

    https.get(url, (response) => {
        if (response.statusCode !== 200) {
            console.warn(`Binary not yet published to GitHub Releases (${response.statusCode}).`);
            console.log(`Skipping auto-download. You will need to build manually: cargo build --release`);
            return;
        }

        response.pipe(file);

        file.on('finish', () => {
            file.close();
            console.log('Download complete.');
            // Note: In a full implementation, we would extract the tar.gz here.
            // For now, this establishes the flow.
        });
    }).on('error', (err) => {
        console.error(`Download failed: ${err.message}`);
    });
}

// Only download if being installed via NPM (not in local dev)
if (!process.env.XPOSE_DEV) {
    download();
} else {
    console.log('XPOSE_DEV detected, skipping download.');
}
