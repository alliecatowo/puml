/**
 * activation.test.js
 *
 * @vscode/test-electron smoke tests for the PUML VS Code extension.
 * Tests run in a real VS Code extension host and verify:
 *   1. Extension activates for .puml files
 *   2. Core commands are registered after activation
 *   3. Live preview opens and renders without crashing
 *   4. LSP restart command is reachable
 *
 * All tests are designed to be non-destructive and self-contained.
 */
'use strict';

const assert = require('node:assert/strict');
const vscode = require('vscode');

const EXTENSION_ID = 'puml.puml-vscode';
const TIMEOUT_MS = 10000;

// Minimal .puml source for smoke renders
const HELLO_PUML = `@startuml
Alice -> Bob: Hello
Bob --> Alice: Hi
@enduml`;

suite('PUML Extension Activation', () => {
  let extension;

  suiteSetup(async () => {
    extension = vscode.extensions.getExtension(EXTENSION_ID);
    if (extension && !extension.isActive) {
      await extension.activate();
    }
  });

  test('extension is present in the extension host', () => {
    assert.ok(extension, `Extension ${EXTENSION_ID} not found in extension host`);
  });

  test('extension activates without error', async () => {
    assert.ok(extension.isActive, 'Extension should be active');
  });

  test('puml.preview.open command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.preview.open'), 'puml.preview.open not registered');
  });

  test('puml.showSvg command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.showSvg'), 'puml.showSvg not registered');
  });

  test('puml.showDiagnostics command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.showDiagnostics'), 'puml.showDiagnostics not registered');
  });

  test('puml.check command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.check'), 'puml.check not registered');
  });

  test('puml.export.svg command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.export.svg'), 'puml.export.svg not registered');
  });

  test('puml.export.png command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.export.png'), 'puml.export.png not registered');
  });

  test('puml.export.source command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.export.source'), 'puml.export.source not registered');
  });

  test('puml.export.json command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.export.json'), 'puml.export.json not registered');
  });

  test('puml.lsp.restart command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.lsp.restart'), 'puml.lsp.restart not registered');
  });

  test('puml.lsp.showOutput command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.lsp.showOutput'), 'puml.lsp.showOutput not registered');
  });

  test('puml.renderScene command is registered', async () => {
    const cmds = await vscode.commands.getCommands(true);
    assert.ok(cmds.includes('puml.renderScene'), 'puml.renderScene not registered');
  });
});

suite('PUML Live Preview', () => {
  let doc;
  let editor;

  suiteSetup(async function () {
    this.timeout(TIMEOUT_MS);
    // Open an untitled puml document
    doc = await vscode.workspace.openTextDocument({
      language: 'puml',
      content: HELLO_PUML,
    });
    editor = await vscode.window.showTextDocument(doc);
  });

  suiteTeardown(async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
  });

  test('active editor is a puml document', () => {
    assert.ok(editor, 'No active editor');
    assert.strictEqual(
      editor.document.languageId,
      'puml',
      `Expected languageId 'puml', got '${editor.document.languageId}'`
    );
  });

  test('puml.preview.open does not throw for a puml document', async function () {
    this.timeout(TIMEOUT_MS);
    // Executing the command should not throw even if the LSP binary is absent
    // (renderer falls back to CLI or returns an error state without crashing).
    try {
      await vscode.commands.executeCommand('puml.preview.open');
    } catch (err) {
      // Surface the error as a test failure, not an unhandled rejection
      assert.fail(`puml.preview.open threw: ${err.message}`);
    }
  });

  test('puml.check does not throw for a puml document', async function () {
    this.timeout(TIMEOUT_MS);
    try {
      await vscode.commands.executeCommand('puml.check');
    } catch (err) {
      assert.fail(`puml.check threw: ${err.message}`);
    }
  });

  test('puml.lsp.restart does not throw', async function () {
    this.timeout(TIMEOUT_MS);
    try {
      await vscode.commands.executeCommand('puml.lsp.restart');
    } catch (err) {
      assert.fail(`puml.lsp.restart threw: ${err.message}`);
    }
  });

  test('puml.lsp.showOutput does not throw', async () => {
    try {
      await vscode.commands.executeCommand('puml.lsp.showOutput');
    } catch (err) {
      assert.fail(`puml.lsp.showOutput threw: ${err.message}`);
    }
  });
});

suite('PUML Show Commands', () => {
  let doc;

  suiteSetup(async function () {
    this.timeout(TIMEOUT_MS);
    doc = await vscode.workspace.openTextDocument({
      language: 'puml',
      content: HELLO_PUML,
    });
    await vscode.window.showTextDocument(doc);
  });

  suiteTeardown(async () => {
    await vscode.commands.executeCommand('workbench.action.closeAllEditors');
  });

  test('puml.showSvg does not throw for a puml document', async function () {
    this.timeout(TIMEOUT_MS);
    try {
      await vscode.commands.executeCommand('puml.showSvg');
    } catch (err) {
      assert.fail(`puml.showSvg threw: ${err.message}`);
    }
  });

  test('puml.showDiagnostics does not throw for a puml document', async function () {
    this.timeout(TIMEOUT_MS);
    try {
      await vscode.commands.executeCommand('puml.showDiagnostics');
    } catch (err) {
      assert.fail(`puml.showDiagnostics threw: ${err.message}`);
    }
  });
});
