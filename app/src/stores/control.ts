// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The control of the lamp beyond the choices of the user: the source the
// lamp follows, the call that shows its color, and the integrations on
// offer. The backend decides (control.rs); the popup shows and asks.

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { ControlStatus } from '@/bindings/ControlStatus';
import { controlGet, controlSetMode } from '@/ipc/commands';
import { mirror } from '@/ipc/events';
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
  function setMode(mode: string | null): Promise<void> {
    return errors.attempt(controlSetMode(mode));
  }

  /** Follows the backend events. Call once per window. */
  function start(): Promise<void> {
    return mirror(status, 'control-changed', controlGet);
  }

  return { status, integration, setMode, start };
});
