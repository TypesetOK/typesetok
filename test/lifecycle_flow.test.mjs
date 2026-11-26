import { test, describe, before } from 'node:test';
import assert from 'node:assert/strict';
import * as fs from 'node:fs';
import * as path from 'node:path';
import { fileURLToPath } from 'node:url';

const __filename = fileURLToPath(import.meta.url);
const __dirname = path.dirname(__filename);
const rootDir = path.resolve(__dirname, '..');

describe('Phase 3 Lifecycle Integration: Window Closing, Safe Exit, and Guards', () => {
  let mainRs;
  let commandsRs;
  let adapterTs;
  let topBarSrc;
  let welcomeSrc;
  let appSrc;

  before(() => {
    mainRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/main.rs'), 'utf-8');
    commandsRs = fs.readFileSync(path.join(rootDir, 'crates/tok-desktop/src/commands.rs'), 'utf-8');
    adapterTs = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/platform/tauriHostAdapter.ts'), 'utf-8');
    topBarSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/TopSystemBar.ts'), 'utf-8');
    welcomeSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/components/WelcomeModal.ts'), 'utf-8');
    appSrc = fs.readFileSync(path.join(rootDir, 'packages/tok-ui/src/app.ts'), 'utf-8');
  });

  test('1. TauriHostAdapter declares closeWindow and quitApp IPC calls', () => {
    assert.ok(adapterTs.includes('closeWindow'), 'Adapter must declare closeWindow');
    assert.ok(adapterTs.includes('quitApp'), 'Adapter must declare quitApp');
    assert.ok(adapterTs.includes('tok_close_window'), 'Adapter must invoke tok_close_window');
    assert.ok(adapterTs.includes('tok_quit_app'), 'Adapter must invoke tok_quit_app');
  });

  test('2. Tauri commands register tok_close_window and tok_quit_app', () => {
    assert.ok(commandsRs.includes('pub fn tok_close_window'), 'commands.rs must define tok_close_window');
    assert.ok(commandsRs.includes('pub fn tok_quit_app'), 'commands.rs must define tok_quit_app');
  });

  test('3. Window close guard checks dirty state and prompts before closing', () => {
    assert.ok(mainRs.includes('WindowEvent::CloseRequested'), 'Main window must handle CloseRequested');
    assert.ok(mainRs.includes('api.prevent_close()'), 'Close event must be prevented when dirty');
    assert.ok(mainRs.includes('is_dirty'), 'Close event must check is_dirty state');
    assert.ok(mainRs.includes('MessageDialogButtons::YesNo'), 'Main must show confirmation dialog when dirty');
  });

  test('4. Duplicate close requests and deadlocks are prevented with active guard', () => {
    assert.ok(commandsRs.includes('tok_set_dirty'), 'commands.rs must expose tok_set_dirty');
    assert.ok(appSrc.includes('isClosingInProgress'), 'Renderer App must guard against re-entrant close requests');
  });

  test('5. TopSystemBar and Menu provide accessible Exit command and shortcut Ctrl+Q', () => {
    assert.ok(topBarSrc.includes("action: 'quit-app'"), 'TopSystemBar MENU_ITEMS must contain quit-app action');
    assert.ok(topBarSrc.includes("shortcut: 'Ctrl+Q'"), 'TopSystemBar must show Ctrl+Q shortcut');
    assert.ok(appSrc.includes("case 'quit-app':"), 'App handleSystemAction must handle quit-app');
    assert.ok(appSrc.includes("case 'exit':"), 'App handleSystemAction must handle exit');
  });

  test('6. Global keyboard listeners support both English (Ctrl+Q) and Hebrew (Ctrl+ק) exit shortcuts', () => {
    assert.ok(appSrc.includes("code === 'KeyQ'"), 'App keydown listener must capture KeyQ');
    assert.ok(appSrc.includes("key === 'ק'"), 'App keydown listener must capture Hebrew letter ק');
  });

  test('7. WelcomeModal allows safe exit and does not trap user when no document is open', () => {
    assert.ok(!welcomeSrc.includes('if (!this.hasOpenDocument) return;'), 'WelcomeModal must not unconditionally block closing');
    assert.ok(welcomeSrc.includes('onExitApp'), 'WelcomeModal must invoke onExitApp callback on close without document');
    assert.ok(welcomeSrc.includes('closeOrExitBtn'), 'WelcomeModal top strip must include close/exit button');
  });

  test('8. Lifecycle state synchronization: window.__tokIsDirty and appInstance', () => {
    assert.ok(appSrc.includes('(window as any).appInstance = this'), 'App must expose appInstance for main window inspection');
    assert.ok(appSrc.includes('(window as any).__tokIsDirty = dirty'), 'setDirty must synchronize __tokIsDirty flag');
  });
});
