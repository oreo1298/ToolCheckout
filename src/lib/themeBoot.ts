import { invoke, isTauri } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { getCurrentWindow } from '@tauri-apps/api/window';
import { ThemeController, type OsAppearance, type Scheme } from './theme';

let lastNative: Scheme | null | undefined;

export const theme = new ThemeController(document.documentElement, (t) => {
  if (!isTauri() || t === lastNative) return;
  lastNative = t;
  getCurrentWindow()
    .setTheme(t)
    .catch(() => {});
});

/** Apply immediately, then follow OS changes pushed by the backend. */
export async function startTheme() {
  theme.apply();
  if (!isTauri()) return;
  try {
    await listen<OsAppearance>('os-appearance', (e) => theme.setOs(e.payload));
    theme.setOs(await invoke<OsAppearance>('get_os_appearance'));
  } catch (err) {
    console.warn('OS appearance unavailable', err);
  }
}
