// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The events the backend sends, to every window or to one, and their payloads.

import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import type { Connection } from '@/bindings/Connection';
import type { ControlStatus } from '@/bindings/ControlStatus';
import type { FirmwareStatus } from '@/bindings/FirmwareStatus';
import type { Frame } from '@/bindings/Frame';
import type { Locale } from '@/bindings/Locale';
import type { Palette } from '@/bindings/Palette';
import type { PluginView } from '@/bindings/PluginView';
import type { Settings } from '@/bindings/Settings';
import type { UpdateStatus } from '@/bindings/UpdateStatus';

interface Events {
  /** The lamp was found or lost (device.rs). */
  'device-connection': Connection;
  /** A message from the lamp. */
  'device-message': Frame;
  /** A report the backend could not decode, or a lamp on another major protocol version. */
  'device-protocol-error': string;
  /** Any window changed the settings. */
  'settings-changed': Settings;
  /** Any window changed the colors. */
  'colors-changed': Palette;
  /** The language of the app changed, through a setting or the system's. */
  'locale-changed': Locale;
  /** The source the lamp follows, the call, the integrations on offer (control.rs). */
  'control-changed': ControlStatus;
  /** The plugins, after any change (plugins/mod.rs). No event carries a token. */
  'plugins-changed': PluginView[];
  'update-status': UpdateStatus;
  'firmware-status': FirmwareStatus;
}

/**
 * Calls `handler` with the payload of each `event` sent to every window or to
 * this one. A listener of any target would also get those sent to another
 * window, as the telemetry for Diagnostics.
 */
export function on<E extends keyof Events>(
  event: E,
  handler: (payload: Events[E]) => void,
): Promise<UnlistenFn> {
  return getCurrentWebviewWindow().listen<Events[E]>(event, ({ payload }) => handler(payload));
}

/**
 * Keeps a value of the backend current in this window, for a window that
 * starts after the last `event`: `apply` gets the answer of `read`, then
 * each event. Every change comes with the event, so once one is heard the
 * events keep the value current, and the answer, which may be older, is
 * dropped.
 */
export async function follow<E extends keyof Events>(
  event: E,
  apply: (payload: Events[E]) => void,
  read: () => Promise<Events[E]>,
): Promise<void> {
  let heard = false;
  await on(event, (payload) => {
    heard = true;
    apply(payload);
  });
  const current = await read();
  if (!heard) {
    apply(current);
  }
}
