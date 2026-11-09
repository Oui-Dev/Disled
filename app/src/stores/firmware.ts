// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Lamp firmware updates. The backend looks for the firmware on offer along
// with the app updates and runs the update; the popup offers it, the guided
// window installs the newest one, and the reinstall window a version of the
// user's choice.

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { FirmwareStatus } from '@/bindings/FirmwareStatus';
import type { Flash } from '@/bindings/Flash';
import {
  firmwareCancel,
  firmwareCheck,
  firmwareInstall,
  firmwareInstallManual,
  firmwareStatus,
} from '@/ipc/commands';
import { follow } from '@/ipc/events';
import { useLampStore } from './lamp';

/** Compares an "X.Y.Z" version with the version a lamp reports. */
function compare(version: string, lamp: readonly number[]): number {
  const parts = version.split('.').map(Number);
  for (let i = 0; i < 3; i++) {
    if (parts[i] !== lamp[i]) {
      return parts[i] - lamp[i];
    }
  }
  return 0;
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
  /** The status of the backend is in, once `start` has read it. */
  const loaded = ref(false);

  /** Whether the connected lamp runs this "X.Y.Z" version. */
  function onLamp(version: string): boolean {
    const info = lamp.info;
    return lamp.connection.connected && info !== null && compare(version, info.firmware) === 0;
  }

  /**
   * The newest firmware 2.x, when the connected lamp runs an older one,
   * whatever its generation: the guided window installs it, a version 1
   * lamp included.
   */
  const newerForLamp = computed(() => {
    const { latest } = status.value;
    const info = lamp.info;
    return latest && info && compare(latest, info.firmware) > 0 ? latest : null;
  });

  /**
   * The same, on offer in the popup, for a version 2 lamp only: a version 1
   * lamp has its own warning, whose button opens the guided window.
   */
  const updateForLamp = computed(() => (lamp.generation === 'v2' ? newerForLamp.value : null));

  /** The status events bring the result, in order with the other changes. */
  async function check(): Promise<void> {
    await firmwareCheck();
  }

  /** An update that does not start shows as failed. */
  async function run(start: Promise<void>): Promise<void> {
    try {
      await start;
    } catch (error) {
      const failed: Flash = { step: 'failed', error: String(error) };
      status.value = { ...status.value, flash: failed };
    }
  }

  /** The newest firmware 2.x, from the guided window. */
  function install(): Promise<void> {
    return run(firmwareInstall());
  }

  /** A version of the list, from the reinstall window. */
  function installManual(version: string): Promise<void> {
    return run(firmwareInstallManual(version));
  }

  /** Follows the backend events. Call once, in the popup or an update window. */
  async function start(): Promise<void> {
    await follow(
      'firmware-status',
      (next) => {
        status.value = next;
      },
      firmwareStatus,
    );
    loaded.value = true;
  }

  return {
    status,
    loaded,
    onLamp,
    newerForLamp,
    updateForLamp,
    check,
    install,
    installManual,
    cancel: firmwareCancel,
    start,
  };
});
