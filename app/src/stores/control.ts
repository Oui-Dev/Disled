// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The control of the lamp beyond the choices of the user: the source the
// lamp follows, the call that shows its color, and the integrations on
// offer. The backend decides (control.rs); the popup shows and asks.

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { ControlStatus } from '@/bindings/ControlStatus';
import { controlGet, controlSetMode } from '@/ipc/commands';
import { follow } from '@/ipc/events';
import { useErrorStore } from './errors';

export const useControlStore = defineStore('control', () => {
  const errors = useErrorStore();

  const status = ref<ControlStatus>({
    mode: null,
    state: null,
    calls: { available: false, in_call: false, imposed: false, back: null },
    integrations: [],
  });

  /** The integration the lamp follows, null in manual mode. */
  const integration = computed(
    () => status.value.integrations.find((known) => known.id === status.value.mode) ?? null,
  );

  /** Follows an integration, or the user with null; a failure becomes the last error. */
  async function setMode(mode: string | null): Promise<void> {
    try {
      await controlSetMode(mode);
      errors.clear();
    } catch (error) {
      errors.reportFailure(error);
    }
  }

  /** Follows the backend events. Call once per window. */
  async function start(): Promise<void> {
    await follow(
      'control-changed',
      (next) => {
        status.value = next;
      },
      controlGet,
    );
  }

  return { status, integration, setMode, start };
});
