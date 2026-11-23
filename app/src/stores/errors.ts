// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The last thing that failed, shown until something the user asked for
// succeeds, the lamp comes back, or the user closes it. The window words
// the cause; the failure's own message stays as it came.

import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { ProtocolError } from '@/bindings/ProtocolError';

export type AppError =
  /** A backend command or a clipboard write failed; `detail` is its message. */
  | { kind: 'backend'; detail: string }
  /** The lamp answered a command with ERROR. */
  | { kind: 'rejected'; command: number; error: ProtocolError }
  /** A report from a version 3 lamp that the backend could not decode. */
  | { kind: 'unreadable'; detail: string };

export const useErrorStore = defineStore('errors', () => {
  const last = ref<AppError | null>(null);

  function report(error: AppError): void {
    last.value = error;
  }

  /** Reports a backend command or a clipboard write that failed. */
  function reportFailure(error: unknown): void {
    report({ kind: 'backend', detail: String(error) });
  }

  function clear(): void {
    last.value = null;
  }

  /**
   * Waits for something the user asked for: a failure becomes the last
   * error, a success clears it. Resolves to the result, undefined after a
   * failure.
   */
  async function attempt<T>(action: Promise<T>): Promise<T | undefined> {
    try {
      const result = await action;
      clear();
      return result;
    } catch (error) {
      reportFailure(error);
      return undefined;
    }
  }

  return { last, report, reportFailure, clear, attempt };
});
