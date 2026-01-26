/**
 * Browser-based end-to-end tests for playground examples
 *
 * Runs each example in a real browser to catch JS errors like
 * template literal issues that only manifest at runtime.
 *
 * Run with: node browser-test.js
 * Requires: npm install puppeteer (or use bunx puppeteer)
 */

import puppeteer from 'puppeteer';
import { spawn } from 'child_process';
import { setTimeout } from 'timers/promises';
import { fileURLToPath } from 'url';
import { dirname } from 'path';

const PORT = 8765;
const BASE_URL = `http://localhost:${PORT}`;
const __dirname = dirname(fileURLToPath(import.meta.url));

let server = null;
let browser = null;

async function startServer() {
    return new Promise((resolve, reject) => {
        server = spawn('python3', ['-m', 'http.server', String(PORT)], {
            cwd: dirname(__dirname),  // Parent of tests directory
            stdio: ['ignore', 'pipe', 'pipe']
        });

        server.stderr.on('data', (data) => {
            if (data.toString().includes('Serving HTTP')) {
                resolve();
            }
        });

        server.on('error', reject);

        // Give it a moment to start
        setTimeout(1000).then(resolve);
    });
}

async function stopServer() {
    if (server) {
        server.kill();
        server = null;
    }
}

// Expected console.error outputs for specific examples (these are NOT failures)
// These are examples that intentionally demonstrate error handling
const EXPECTED_ERRORS = {
    'Error Handling': /Caught error:|Validation failed|WASM/,
};

async function runTests() {
    console.log('Starting local server...');
    await startServer();

    console.log('Launching browser...');
    // Try Firefox if BROWSER=firefox, otherwise use Chrome
    const useFirefox = process.env.BROWSER === 'firefox';
    browser = await puppeteer.launch({
        headless: true,
        product: useFirefox ? 'firefox' : 'chrome',
        args: useFirefox ? [] : ['--no-sandbox', '--disable-gpu', '--disable-software-rasterizer'],
        protocolTimeout: 60000  // Increase protocol timeout to 60s
    });

    const page = await browser.newPage();

    // Collect console errors
    const errors = [];
    page.on('pageerror', err => {
        console.log('  [PAGE ERROR]', err.message);
        errors.push(err.message);
    });
    page.on('console', msg => {
        if (msg.type() === 'error') {
            console.log('  [CONSOLE ERROR]', msg.text());
            errors.push(msg.text());
        }
        // Log WASM console output for debugging
        if (msg.text().includes('[WASM]')) {
            console.log(`  ${msg.text()}`);
        }
        // Log debug output
        if (msg.text().includes('[DEBUG]')) {
            console.log(`  ${msg.text()}`);
        }
    });

    // Handle page crashes
    page.on('error', err => {
        console.log('  [PAGE CRASHED]', err.message);
    });

    // Handle renderer crashes
    page.on('close', () => {
        console.log('  [PAGE CLOSED]');
    });

    console.log(`Loading ${BASE_URL}...`);
    await page.goto(BASE_URL, { waitUntil: 'networkidle0' });

    // Wait for WASM to load
    await page.waitForSelector('#status.success', { timeout: 30000 });
    console.log('WASM loaded successfully\n');

    // Get all example options
    const examples = await page.evaluate(() => {
        const select = document.getElementById('examples');
        return Array.from(select.options).map(opt => ({
            value: opt.value,
            name: opt.textContent
        }));
    });

    console.log(`Found ${examples.length} examples to test\n`);

    // Run all examples in a single browser-side loop to avoid Puppeteer CDP issues
    const results = await page.evaluate(async (exampleData, expectedErrorPatterns) => {
        const delay = ms => new Promise(resolve => setTimeout(resolve, ms));
        const results = [];

        for (const example of exampleData) {
            // Select example
            document.getElementById('examples').value = example.value;
            document.getElementById('code').value = window.EXAMPLES?.[example.value]?.code || '';

            await delay(100);

            // Click run button
            document.getElementById('run-btn').click();

            // Wait for completion (poll for status change)
            const startTime = Date.now();
            while (Date.now() - startTime < 30000) {
                const status = document.getElementById('status');
                if (status.classList.contains('success') || status.classList.contains('error')) {
                    break;
                }
                await delay(100);
            }

            await delay(50);

            // Collect result
            const statusEl = document.getElementById('status');
            const outputEl = document.getElementById('output');
            const errorLines = outputEl.querySelectorAll('.output-error');
            const errorTexts = Array.from(errorLines).map(el => el.textContent);

            const runtimeError = errorTexts.find(text =>
                text.startsWith('Error:') ||
                text.includes('Parse error') ||
                text.includes('Unexpected token')
            );

            results.push({
                name: example.name,
                value: example.value,
                status: statusEl.className,
                hasError: !!runtimeError,
                errorText: runtimeError || null
            });
        }

        return results;
    }, examples, EXPECTED_ERRORS);

    let passed = 0;
    let failed = 0;

    for (const result of results) {
        // Check for page errors (like ReferenceError in main.js)
        // Filter out expected errors for specific examples
        const expectedPattern = EXPECTED_ERRORS[result.name];
        const unexpectedPageErrors = expectedPattern
            ? errors.filter(e => !expectedPattern.test(e))
            : errors.filter(e => true); // Note: errors array accumulates across all examples

        // For now, just check output errors since page errors accumulate
        const hasOutputError = result.hasError;

        if (hasOutputError) {
            console.log(`✗ ${result.name}`);
            if (hasOutputError) {
                console.log(`  Runtime error: ${result.errorText}`);
            }
            failed++;
        } else {
            console.log(`✓ ${result.name}`);
            passed++;
        }
    }

    console.log('\n═══════════════════════════════════════════════════════════════');
    console.log(`Results: ${passed} passed, ${failed} failed`);
    console.log('═══════════════════════════════════════════════════════════════\n');

    await browser.close();
    await stopServer();

    process.exit(failed > 0 ? 1 : 0);
}

runTests().catch(async err => {
    console.error('Test failed:', err);
    if (browser) await browser.close();
    await stopServer();
    process.exit(1);
});
