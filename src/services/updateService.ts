import { invoke } from '@tauri-apps/api/core';

/**
 * Check GitHub releases for a newer version via Velopack.
 *
 * If an update is found it is downloaded and applied, and the app restarts
 * (so this call may not return in that case). Resolves to `true` when an
 * update was applied, `false` when already up to date.
 */
export async function checkAndInstallUpdate(): Promise<boolean> {
  return invoke<boolean>('check_and_install_update');
}
