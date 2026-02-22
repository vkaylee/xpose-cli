const assert = require('assert');
const fs = require('fs');
const path = require('path');
const crypto = require('crypto');
const { execSync } = require('child_process');
const { getFileContent } = require('./install');

// Setup temporary test environment
const TEST_DIR = path.join(__dirname, '..', 'test_tmp');
const BIN_DIR = path.join(TEST_DIR, 'bin');
const DUMMY_BIN = 'xpose';
const DUMMY_CONTENT = 'dummy binary content';
const ARCHIVE_NAME = 'test-target.tar.gz';

if (!fs.existsSync(BIN_DIR)) {
    fs.mkdirSync(BIN_DIR, { recursive: true });
}

async function testExtraction() {
    console.log('Running testExtraction...');

    const archivePath = path.join(BIN_DIR, ARCHIVE_NAME);
    const dummyBinPath = path.join(TEST_DIR, DUMMY_BIN);

    // 1. Create a dummy binary
    fs.writeFileSync(dummyBinPath, DUMMY_CONTENT);

    // 2. Archive it
    execSync(`tar -czf "${archivePath}" -C "${TEST_DIR}" "${DUMMY_BIN}"`);
    fs.unlinkSync(dummyBinPath); // Remove original dummy

    // 3. Run extraction logic (simulating the end of install.js download)
    console.log(`Extracting ${ARCHIVE_NAME}...`);
    try {
        execSync(`tar -xzf "${archivePath}" -C "${BIN_DIR}"`);

        const extractedPath = path.join(BIN_DIR, DUMMY_BIN);
        assert(fs.existsSync(extractedPath), 'Extracted binary should exist');

        const content = fs.readFileSync(extractedPath, 'utf8');
        assert.strictEqual(content, DUMMY_CONTENT, 'Extracted content should match');

        if (process.platform !== 'win32') {
            fs.chmodSync(extractedPath, 0o755);
            const stats = fs.statSync(extractedPath);
            assert((stats.mode & 0o777) === 0o755, 'Binary should have correct permissions');
        }

        console.log('✅ testExtraction passed');
    } catch (e) {
        console.error('❌ testExtraction failed:', e.message);
        process.exit(1);
    } finally {
        // Cleanup
        if (fs.existsSync(archivePath)) fs.unlinkSync(archivePath);
        const extractedPath = path.join(BIN_DIR, DUMMY_BIN);
        if (fs.existsSync(extractedPath)) fs.unlinkSync(extractedPath);
    }
}

async function testGetFileContent404() {
    console.log('Running testGetFileContent404...');
    const url = 'https://github.com/vkaylee/xpose-cli/releases/download/v0.3.2/NON_EXISTENT_FILE.sha256';
    try {
        await getFileContent(url);
        console.error('❌ testGetFileContent404 FAILED: Expected rejection for 404, but it resolved.');
        process.exit(1);
    } catch (e) {
        if (e.message.includes('Status: 404')) {
            console.log('✅ testGetFileContent404 passed: Correctly rejected with Status: 404');
        } else {
            console.error('❌ testGetFileContent404 FAILED: Unexpected error message:', e.message);
            process.exit(1);
        }
    }
}

async function runTests() {
    try {
        await testExtraction();
        await testGetFileContent404();
        console.log('\n✨ All installation tests passed!');
    } finally {
        // Final cleanup of test directory
        if (fs.existsSync(TEST_DIR)) {
            fs.rmSync(TEST_DIR, { recursive: true, force: true });
        }
    }
}

runTests().catch(err => {
    console.error(err);
    process.exit(1);
});
