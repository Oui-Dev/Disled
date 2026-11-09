// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The last thing that failed, shown until something the user asked for
// succeeds, the lamp comes back, or the user closes it. The window words
// the cause; the backend's own message stays as it came.

import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { ProtocolError } from '@/bindings/ProtocolError';

export type AppError =
  /** A backend command failed; `detail` is its message. */
  | { kind: 'backend'; detail: string }
  /** The lamp answered a command with ERROR. */
  | { kind: 'rejected'; command: number; error: ProtocolError }
  /** A report the backend could not decode, or a lamp on another major protocol version. */
  | { kind: 'unreadable'; detail: string };

export const useErrorStore = defineStore('errors', () => {
  const last = ref<AppError | null>(null);

  function report(error: AppError): void {
    last.value = error;
  }

  /** Reports a rejected backend command. */
  function reportFailure(error: unknown): void {
    report({ kind: 'backend', detail: String(error) });
  }

  function clear(): void {
    last.value = null;
  }

  return { last, report, reportFailure, clear };
});
