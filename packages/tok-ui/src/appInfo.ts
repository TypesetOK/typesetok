import { getHostBridge } from './platform/hostBridge.js';

let versionPromise: Promise<string> | null = null;

/** The running app's version from the host process, or '' when unavailable. */
export function getAppVersion(): Promise<string> {
  if (!versionPromise) {
    versionPromise = (async () => {
      try {
        const host = getHostBridge();
        if (!host.has('getAppInfo')) return '';
        const info = await host.getAppInfo();
        return typeof info?.version === 'string' ? info.version : '';
      } catch {
        return '';
      }
    })();
  }
  return versionPromise;
}

/** Fills `el` with the version once it is known, formatted by `format`. */
export function fillAppVersion(el: HTMLElement, format: (v: string) => string = (v) => `v${v}`): void {
  getAppVersion().then((v) => {
    if (v) el.textContent = format(v);
  });
}
