// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// App updates. The backend checks GitHub at start-up and then once a day,
// and the popup shows the result and installs the update on request.

import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { UpdateStatus } from '@/bindings/UpdateStatus';
import { updateCheck, updateInstall, updateStatus } from '@/ipc/commands';
import { follow } from '@/ipc/events';

export const useUpdatesStore = defineStore('updates', () => {
  const status = ref<UpdateStatus>({
    checking: false,
    available: null,
    progress: null,
    error: null,
    checked_at: null,
  });

  /** The status events bring the result, in order with the other changes. */
  async function check(): Promise<void> {
    await updateCheck();
  }

  /** Downloads and installs the update; the app restarts once it is done. */
  async function install(): Promise<void> {
    try {
      await updateInstall();
    } catch {
      // The status event carries the error.
    }
  }

  /** Follows the backend events. Call once, in the popup. */
  async function start(): Promise<void> {
    await follow(
      'update-status',
      (next) => {
        status.value = next;
      },
      updateStatus,
    );
  }

  return { status, check, install, start };
});
