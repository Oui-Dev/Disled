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
import { firmwareCancel, firmwareCheck, firmwareInstall, firmwareStatus } from '@/ipc/commands';
import { mirror } from '@/ipc/events';
import { compare } from '@/utils/version';
import { useLampStore } from './lamp';

export const useFirmwareStore = defineStore('firmware', () => {
  const lamp = useLampStore();

  const status = ref<FirmwareStatus>({
    checking: false,
    versions: [],
    error: null,
    checked_at: null,
    target: null,
    flash: { step: 'idle' },
  });
  /** The status of the backend is in, once `start` has read it. */
  const loaded = ref(false);

  /** The newest firmware 2.x, which the update buttons install. */
  const latest = computed(() => status.value.versions[0] ?? null);

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
    const newest = latest.value;
    const info = lamp.info;
    return newest && info && compare(newest, info.firmware) > 0 ? newest : null;
  });

  /**
   * The same, on offer in the popup, for a version 2 lamp only: a version 1
   * lamp has its own warning, whose button opens the guided window.
   */
  const updateForLamp = computed(() => (lamp.generation === 'v2' ? newerForLamp.value : null));

  /** A version of the list; an update that does not start shows as failed. */
  async function install(version: string): Promise<void> {
    try {
      await firmwareInstall(version);
    } catch (error) {
      const failed: Flash = { step: 'failed', error: String(error) };
      status.value = { ...status.value, flash: failed };
    }
  }

  /** Follows the backend events. Call once, in the popup or an update window. */
  async function start(): Promise<void> {
    await mirror(status, 'firmware-status', firmwareStatus);
    loaded.value = true;
  }

  return {
    status,
    loaded,
    latest,
    onLamp,
    newerForLamp,
    updateForLamp,
    // The status events bring the result, in order with the other changes.
    check: firmwareCheck,
    install,
    cancel: firmwareCancel,
    start,
  };
});
