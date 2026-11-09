// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The Tauri commands of the backend (src-tauri/src/main.rs), typed. Failed
// commands reject with the backend's error message.

import { invoke } from '@tauri-apps/api/core';
import type { Command } from '@/bindings/Command';
import type { Connection } from '@/bindings/Connection';
import type { FirmwareStatus } from '@/bindings/FirmwareStatus';
import type { LampChoice } from '@/bindings/LampChoice';
import type { Link } from '@/bindings/Link';
import type { Locale } from '@/bindings/Locale';
import type { Settings } from '@/bindings/Settings';
import type { SettingsPatch } from '@/bindings/SettingsPatch';
import type { UpdateStatus } from '@/bindings/UpdateStatus';

/** The connection now, for a window that opens after the first event. */
export function deviceConnection(): Promise<Connection> {
  return invoke('device_connection');
}

/** Queues a command for the lamp; replies arrive as `device-message` events. */
export function deviceSend(command: Command): Promise<void> {
  return invoke('device_send', { command });
}

/** Tells the backend what the user chose for the lamp: off, or a color slot. */
export function lampChoose(choice: LampChoice): Promise<void> {
  return invoke('lamp_choose', { choice });
}

/** Writes a recording to a `.csv` file chosen in a save dialog. */
export function saveCsv(path: string, contents: string): Promise<void> {
  return invoke('save_csv', { path, contents });
}

export function settingsGet(): Promise<Settings> {
  return invoke('settings_get');
}

/** Saves settings; the backend applies what changed to the app and to the lamp. */
export function settingsSet(patch: SettingsPatch): Promise<Settings> {
  return invoke('settings_set', { patch });
}

/** The language of the app now: the setting, or the system's. */
export function localeGet(): Promise<Locale> {
  return invoke('locale_get');
}

export function autostartGet(): Promise<boolean> {
  return invoke('autostart_get');
}

/** Turns start at login on or off; resolves to the state now in force. */
export function autostartSet(enabled: boolean): Promise<boolean> {
  return invoke('autostart_set', { enabled });
}

export function updateStatus(): Promise<UpdateStatus> {
  return invoke('update_status');
}

/** `update-status` events bring the result. */
export function updateCheck(): Promise<void> {
  return invoke('update_check');
}

/** Rejects on failure; on success the app restarts. */
export function updateInstall(): Promise<void> {
  return invoke('update_install');
}

export function firmwareStatus(): Promise<FirmwareStatus> {
  return invoke('firmware_status');
}

/** `firmware-status` events bring the result. */
export function firmwareCheck(): Promise<void> {
  return invoke('firmware_check');
}

/** Starts a lamp update; `firmware-status` events follow it. */
export function firmwareInstall(): Promise<void> {
  return invoke('firmware_install');
}

/** Stops an update that waits for the bootloader. */
export function firmwareCancel(): Promise<void> {
  return invoke('firmware_cancel');
}

export function openDiagnostics(): Promise<void> {
  return invoke('open_diagnostics');
}

export function openLampUpdate(): Promise<void> {
  return invoke('open_lamp_update');
}

/**
 * Fits the popup to this height (CSS pixels), against the tray icon;
 * resolves with the height it got, shorter on a small screen.
 */
export function popupResize(height: number): Promise<number> {
  return invoke('popup_resize', { height });
}

export function popupHide(): Promise<void> {
  return invoke('popup_hide');
}

/** Opens one of the pages the app links to in the browser. */
export function openLink(link: Link): Promise<void> {
  return invoke('open_link', { link });
}

export function quit(): Promise<void> {
  return invoke('quit');
}
