/**
 * tests/suite/index.js
 *
 * @vscode/test-electron test runner entry point.
 * Discovers and runs all *.test.js files in this directory.
 *
 * Run with:
 *   npm run test:electron
 *
 * Requires a display (Xvfb on Linux CI) and @vscode/test-electron installed.
 */
'use strict';

const path = require('node:path');
const Mocha = require('mocha');
const glob = require('glob');

function run() {
  const mocha = new Mocha({ ui: 'bdd', color: true, timeout: 15000 });

  const testsRoot = __dirname;
  const files = glob.sync('**/*.test.js', { cwd: testsRoot });

  for (const f of files) {
    mocha.addFile(path.resolve(testsRoot, f));
  }

  return new Promise((resolve, reject) => {
    mocha.run((failures) => {
      if (failures > 0) {
        reject(new Error(`${failures} test(s) failed.`));
      } else {
        resolve();
      }
    });
  });
}

module.exports = { run };
