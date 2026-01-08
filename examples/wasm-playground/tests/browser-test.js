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

async function runTests() {
    console.log('Starting local server...');
    await startServer();

    console.log('Launching browser...');
    browser = await puppeteer.launch({
        headless: true,
        args: ['--no-sandbox']
    });

    const page = await browser.newPage();

    // Collect console errors
    const errors = [];
    page.on('pageerror', err => errors.push(err.message));
    page.on('console', msg => {
        if (msg.type() === 'error') {
            errors.push(msg.text());
        }
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

    let passed = 0;
    let failed = 0;

    for (const example of examples) {
        errors.length = 0; // Clear errors

        // Select the example
        await page.select('#examples', example.value);
        await setTimeout(100);

        // Click Run
        await page.click('#run-btn');

        // Wait for execution to complete (success or error)
        try {
            await page.waitForFunction(
                () => {
                    const status = document.getElementById('status');
                    return status.classList.contains('success') || status.classList.contains('error');
                },
                { timeout: 10000 }
            );
        } catch (e) {
            // Get current status for debugging
            const status = await page.evaluate(() => document.getElementById('status').textContent);
            throw new Error(`Timeout waiting for completion. Status: ${status}`);
        }
        await setTimeout(100);

        // Check for errors in output (parse errors, runtime errors like TypeError, etc.)
        // Note: console.error() output also has .output-error class, but runtime errors
        // are prefixed with "Error:" by the playground's displayOutput function
        const output = await page.evaluate(() => {
            const outputEl = document.getElementById('output');
            const errorLines = outputEl.querySelectorAll('.output-error');
            const errorTexts = Array.from(errorLines).map(el => el.textContent);
            // Only flag as error if it's a real runtime error (prefixed with "Error:")
            // not just console.error() output from the example code
            const runtimeError = errorTexts.find(text =>
                text.startsWith('Error:') ||
                text.includes('Parse error') ||
                text.includes('Unexpected token')
            );
            return {
                hasError: !!runtimeError,
                errorText: runtimeError || null,
                text: outputEl.textContent
            };
        });

        // Check for page errors (like ReferenceError in main.js)
        const hasPageError = errors.length > 0;
        const hasOutputError = output.hasError;

        if (hasPageError || hasOutputError) {
            console.log(`✗ ${example.name}`);
            if (hasPageError) {
                console.log(`  Page error: ${errors[0]}`);
            }
            if (hasOutputError) {
                console.log(`  Runtime error: ${output.errorText}`);
            }
            failed++;
        } else {
            console.log(`✓ ${example.name}`);
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
