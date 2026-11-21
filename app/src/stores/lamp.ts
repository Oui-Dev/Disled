// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The lamp as one window sees it: the connection, its identity (INFO) and
// its state (STATE). The lamp owns its state, so the window only shows the
// last STATE message and sends commands.

import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import type { Command } from '@/bindings/Command';
import type { Connection } from '@/bindings/Connection';
import type { Info } from '@/bindings/Info';
import type { LampChoice } from '@/bindings/LampChoice';
import type { LampState } from '@/bindings/LampState';
import { deviceConnection, deviceSend, lampChoose } from '@/ipc/commands';
import { follow, on } from '@/ipc/events';
import { useErrorStore } from './errors';

export const useLampStore = defineStore('lamp', () => {
  const errors = useErrorStore();

  const connection = ref<Connection>({
    connected: false,
    debug_build: false,
    serial: null,
    generation: null,
  });
  const info = ref<Info | null>(null);
  const state = ref<LampState | null>(null);

  /**
   * What the protocol of the lamp lets the app do, null until it answers: a
   * version 1 lamp and a lamp newer than the app are locked, a version 2
   * lamp is driven.
   */
  const generation = computed(() => connection.value.generation);
  /** A lamp the app does not drive: its controls stay greyed. */
  const locked = computed(() => generation.value === 'v1' || generation.value === 'newer');
  /** A version 2 lamp whose state is known, which the controls drive. */
  const ready = computed(
    () => connection.value.connected && generation.value === 'v2' && state.value !== null,
  );

  /**
   * Sends a command; a failure becomes the last error. A command the user
   * asked for clears it once sent. One the app sends on its own, to poll or
   * to start a stream again, leaves it on show, since it says nothing of it.
   */
  async function send(command: Command, { automatic = false } = {}): Promise<void> {
    try {
      await deviceSend(command);
      if (!automatic) {
        errors.clear();
      }
    } catch (error) {
      errors.reportFailure(error);
    }
  }

  /**
   * The user turns the lamp off, or on with a color slot: the lamp's own
   * state, which this computer does not save. A failure becomes the last error.
   */
  async function choose(choice: LampChoice): Promise<void> {
    try {
      await lampChoose(choice);
      errors.clear();
    } catch (error) {
      errors.reportFailure(error);
    }
  }

  function restart(): Promise<void> {
    return send({ kind: 'reset' });
  }

  /** Follows the backend events. Call once per window. */
  async function start(): Promise<void> {
    // Telemetry, diagnostics and self-test results go to their own stores.
    await on('device-message', ({ message }) => {
      switch (message.kind) {
        case 'info':
          info.value = message;
          break;
        case 'state':
          state.value = message;
          break;
        case 'error':
          errors.report({ kind: 'rejected', command: message.command, error: message.error });
          break;
      }
    });
    await on('device-protocol-error', (detail) => {
      errors.report({ kind: 'unreadable', detail });
    });
    await follow(
      'device-connection',
      (next) => {
        connection.value = next;
        if (next.connected) {
          errors.clear();
        } else {
          info.value = null;
          state.value = null;
        }
      },
      deviceConnection,
    );
    if (connection.value.connected) {
      await send({ kind: 'get_info' }, { automatic: true });
      await send({ kind: 'get_state' }, { automatic: true });
    }
  }

  return {
    connection,
    info,
    state,
    generation,
    locked,
    ready,
    send,
    choose,
    restart,
    start,
  };
});
