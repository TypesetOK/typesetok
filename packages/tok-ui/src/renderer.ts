import { TypesetOkApp } from './app';
import { getHostBridge } from './platform/hostBridge.js';

// Bootstrap TypesetOK Workbench
window.addEventListener('DOMContentLoaded', () => {
  const appContainer = document.getElementById('app');
  if (!appContainer) {
    console.error('Missing #app container');
    return;
  }

  const app = new TypesetOkApp(appContainer);
  (window as any).tokApp = app;

  // Startup milestone (read by the opt-in startup trace in tok-electron/main.ts)
  performance.mark('tok-ui-built');

  const runAction = (action: unknown, data?: unknown) => {
    if (typeof action === 'string' && action) app.handleSystemAction(action, data);
  };

  // Hook into desktop host bridge if running inside Electron or Tauri
  const host = getHostBridge();
  if (host.isDesktop) {
    // Native menu actions from the host process
    host.onEvent((payload: any) => {
      if (payload && typeof payload === 'object') runAction(payload.action, payload.data);
    });
    // Check if launched with a file from OS file association or CLI
    if (host.has('getStartupFile')) {
      host.getStartupFile().then((file: string | null) => {
        if (file) {
          app.openAnyFile(file);
        }
      }).catch(() => {});
    }
  } else {
    console.log('[TOK] Running in standalone web preview mode');
  }

  // Global window listener for custom dispatch
  window.addEventListener('tok-action', ((e: CustomEvent) => {
    runAction(e.detail?.action, e.detail?.data);
  }) as EventListener);
});