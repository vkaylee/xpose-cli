#!/usr/bin/env node

const fs = require('fs');
const path = require('path');
const os = require('os');
const https = require('https');
const crypto = require('crypto');

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

function fetch(url) {
    return new Promise((resolve, reject) => {
        https.get(url, (res) => {
            if (res.statusCode === 301 || res.statusCode === 302) {
                return fetch(res.headers.location).then(resolve).catch(reject);
            }
            if (res.statusCode !== 200) {
                return reject(new Error(`Failed to fetch ${url} (Status: ${res.statusCode})`));
            }
            resolve(res);
        }).on('error', reject);
    });
}

function getFileContent(url) {
    return new Promise((resolve, reject) => {
        https.get(url, (res) => {
            if (res.statusCode === 301 || res.statusCode === 302) {
                return getFileContent(res.headers.location).then(resolve).catch(reject);
            }
            let data = '';
            res.on('data', hunk => data += hunk);
            res.on('end', () => resolve(data.trim()));
            res.on('error', reject);
        });
    });
}

async function download() {
    let cliProgress;
    try {
        cliProgress = require('cli-progress');
    } catch (e) {
        console.log('Progress bar library not found, skipping visual progress.');
    }

    if (!fs.existsSync(BIN_DIR)) {
        fs.mkdirSync(BIN_DIR, { recursive: true });
    }

    const releaseName = getReleaseName();
    const url = `${BASE_URL}/${releaseName}`;
    const checksumUrl = `${url}.sha256`;
    const dest = path.join(BIN_DIR, releaseName);

    try {
        console.log(`Verifying release for ${releaseName}...`);

        let expectedChecksum;
        try {
            const checksumContent = await getFileContent(checksumUrl);
            expectedChecksum = checksumContent.split(' ')[0];
        } catch (e) {
            console.warn(`⚠️  Checksum file not found at ${checksumUrl}. Skipping verification.`);
        }

        console.log(`Downloading xpose binary...`);
        const response = await fetch(url);
        const totalSize = parseInt(response.headers['content-length'], 10);

        const file = fs.createWriteStream(dest);
        const hash = crypto.createHash('sha256');

        const progressBar = cliProgress ? new cliProgress.SingleBar({
            format: 'Progress |{bar}| {percentage}% | {value}/{total} bytes',
            barCompleteChar: '\u2588',
            barIncompleteChar: '\u2591',
            hideCursor: true
        }) : null;

        if (progressBar) progressBar.start(totalSize, 0);

        let downloadedSize = 0;
        response.on('data', (chunk) => {
            downloadedSize += chunk.length;
            if (progressBar) progressBar.update(downloadedSize);
            hash.update(chunk);
        });

        response.pipe(file);

        await new Promise((resolve, reject) => {
            file.on('finish', () => {
                if (progressBar) progressBar.stop();
                file.close();
                resolve();
            });
            file.on('error', reject);
        });

        if (expectedChecksum) {
            const actualChecksum = hash.digest('hex');
            if (actualChecksum === expectedChecksum) {
                console.log('✅ Integrity verified: SHA256 matches.');
            } else {
                console.error('❌ Integrity check FAILED: Checksum mismatch!');
                fs.unlinkSync(dest);
                process.exit(1);
            }
        }

        console.log(`Successfully installed to ${dest}`);
        console.log('Note: Run "xpose" to start the tunnel.');

    } catch (err) {
        if (err.message.includes('404')) {
            console.warn(`Binary not yet published to GitHub Releases.`);
            console.log(`Skipping auto-download. You can build manually: cargo build --release`);
        } else {
            console.error(`Download failed: ${err.message}`);
        }
    }
}

// Only download if being installed via NPM (not in local dev)
if (require.main === module) {
    if (!process.env.XPOSE_DEV) {
        download();
    } else {
        console.log('XPOSE_DEV detected, skipping download.');
    }
}

module.exports = { download, getReleaseName };
