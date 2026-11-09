// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Lamp firmware updates. The backend looks for a new firmware along with the
// app updates and runs the update; the popup offers it, and the lamp update
// window guides it.

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { FirmwareStatus } from '@/bindings/FirmwareStatus';
import { firmwareCancel, firmwareCheck, firmwareInstall, firmwareStatus } from '@/ipc/commands';
import { follow } from '@/ipc/events';
import { useLampStore } from './lamp';

/** Whether `latest`, an "X.Y.Z" string, is newer than the version a lamp reports. */
function isNewer(latest: string, current: readonly number[]): boolean {
  const next = latest.split('.').map(Number);
  for (let i = 0; i < 3; i++) {
    if (next[i] !== current[i]) {
      return next[i] > current[i];
    }
  }
  return false;
}

export const useFirmwareStore = defineStore('firmware', () => {
  const lamp = useLampStore();

  const status = ref<FirmwareStatus>({
    checking: false,
    latest: null,
    versions: [],
    error: null,
    checked_at: null,
    target: null,
    flash: { step: 'idle' },
  });

  /** The newest firmware, when the connected lamp runs an older one. */
  const updateForLamp = computed(() => {
    const { latest } = status.value;
    const info = lamp.info;
    return latest && info && isNewer(latest, info.firmware) ? latest : null;
  });

  /** The status events bring the result, in order with the other changes. */
  async function check(): Promise<void> {
    await firmwareCheck();
  }

  async function install(): Promise<void> {
    try {
      await firmwareInstall();
    } catch (error) {
      status.value = { ...status.value, flash: { step: 'failed', error: String(error) } };
    }
  }

  /** Follows the backend events. Call once, in the popup or the update window. */
  async function start(): Promise<void> {
    await follow(
      'firmware-status',
      (next) => {
        status.value = next;
      },
      firmwareStatus,
    );
  }

  return { status, updateForLamp, check, install, cancel: firmwareCancel, start };
});
