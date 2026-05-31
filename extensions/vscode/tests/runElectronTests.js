/**
 * runElectronTests.js
 *
 * Launches a VS Code Extension Host for the @vscode/test-electron smoke suite.
 * Run via `npm run test:electron`.
 *
 * Requirements:
 *   - @vscode/test-electron installed (devDependency)
 *   - A display available (Xvfb on headless CI)
 *
 * Usage:
 *   npm run test:electron
 */
'use strict';

const path = require('node:path');

async function main() {
  let runTests;
  try {
    ({ runTests } = require('@vscode/test-electron'));
  } catch {
    console.error(
      '[runElectronTests] @vscode/test-electron is not installed.\n' +
      'Run: npm install --save-dev @vscode/test-electron mocha glob\n' +
      'Then: npm run test:electron'
    );
    process.exit(1);
  }

  try {
    await runTests({
      // VS Code version to test against. 'stable' downloads the latest stable release.
      version: 'stable',
      extensionDevelopmentPath: path.resolve(__dirname, '..'),
      extensionTestsPath: path.resolve(__dirname, 'suite', 'index'),
      // Suppress VS Code's UI noise in CI
      launchArgs: ['--disable-extensions'],
    });
  } catch (err) {
    console.error('Electron test run failed:', err);
    process.exit(1);
  }
}

main();
