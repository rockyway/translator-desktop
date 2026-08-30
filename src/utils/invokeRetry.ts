import { invoke, InvokeArgs } from '@tauri-apps/api/core';

const DEFAULT_MAX_RETRIES = 10;
const DEFAULT_RETRY_DELAY_MS = 150;

function delay(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}

/**
 * Invokes a Tauri command, retrying briefly on failure.
 *
 * Config-declared windows (see `tauri.conf.json`) start loading their frontend
 * before the Rust `setup()` hook finishes registering backend state like `DbState`
 * (see the `.manage(PopupTextState::default())` comment in `lib.rs` for the same
 * race). A command invoked at mount before its required state is managed is
 * rejected with "state not managed" - retrying briefly lets it succeed once the
 * backend catches up, instead of silently falling back to defaults for the
 * whole session.
 *
 * Only use this for read-only/idempotent commands invoked at component mount,
 * where a transient retry is harmless.
 */
export async function invokeWithStartupRetry<T>(
  command: string,
  args?: InvokeArgs,
  maxRetries: number = DEFAULT_MAX_RETRIES,
  retryDelayMs: number = DEFAULT_RETRY_DELAY_MS
): Promise<T> {
  let lastError: unknown;
  for (let attempt = 0; attempt <= maxRetries; attempt++) {
    try {
      return await invoke<T>(command, args);
    } catch (error) {
      lastError = error;
      if (attempt < maxRetries) {
        await delay(retryDelayMs);
      }
    }
  }
  throw lastError;
}
