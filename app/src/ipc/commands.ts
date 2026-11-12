// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The Tauri commands of the backend (src-tauri/src/main.rs), typed. Failed
// commands reject with the backend's error message.

import { invoke } from '@tauri-apps/api/core';
import type { Command } from '@/bindings/Command';
import type { Connection } from '@/bindings/Connection';
import type { ControlStatus } from '@/bindings/ControlStatus';
import type { FilledBlock } from '@/bindings/FilledBlock';
import type { FirmwareStatus } from '@/bindings/FirmwareStatus';
import type { LampChoice } from '@/bindings/LampChoice';
import type { Link } from '@/bindings/Link';
import type { Locale } from '@/bindings/Locale';
import type { Palette } from '@/bindings/Palette';
import type { PluginView } from '@/bindings/PluginView';
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

/**
 * Tells the backend what the user chose for the lamp: off, or on, with a
 * color slot in manual mode, with the color of the integration otherwise.
 */
export function lampChoose(choice: LampChoice): Promise<void> {
  return invoke('lamp_choose', { choice });
}

/** What the windows show of the control, for a window that starts. */
export function controlGet(): Promise<ControlStatus> {
  return invoke('control_get');
}

/** The source the lamp follows: an integration on offer, or null for the user. */
export function controlSetMode(mode: string | null): Promise<void> {
  return invoke('control_set_mode', { mode });
}

/** The plugins, read again from their folder. */
export function pluginsList(): Promise<PluginView[]> {
  return invoke('plugins_list');
}

/** The block of the installation note, with the address and the token. */
export function pluginNote(id: string): Promise<FilledBlock | null> {
  return invoke('plugin_note', { id });
}

/** Writes the block of the note to the clipboard, which the app never reads. */
export function pluginCopyNote(id: string): Promise<void> {
  return invoke('plugin_copy_note', { id });
}

/** "Enable" in the question, for the fingerprint it showed. */
export function pluginApprove(id: string, fingerprint: string): Promise<void> {
  return invoke('plugin_approve', { id, fingerprint });
}

/** Turning a plugin on needs the approval of its files as they are. */
export function pluginSetEnabled(id: string, enabled: boolean): Promise<void> {
  return invoke('plugin_set_enabled', { id, enabled });
}

export function pluginSetColor(id: string, state: string, slot: number): Promise<void> {
  return invoke('plugin_set_color', { id, state, slot });
}

export function pluginRegenerateToken(id: string): Promise<void> {
  return invoke('plugin_regenerate_token', { id });
}

export function pluginRemove(id: string): Promise<void> {
  return invoke('plugin_remove', { id });
}

/** Imports the plugin of a zip file; rejects with the reason of a refusal. */
export function pluginsImport(path: string): Promise<string> {
  return invoke('plugins_import', { path });
}

export function pluginsOpenFolder(): Promise<void> {
  return invoke('plugins_open_folder');
}

export function openIntegrations(): Promise<void> {
  return invoke('open_integrations');
}

export function colorsGet(): Promise<Palette> {
  return invoke('colors_get');
}

/**
 * Saves a custom color, with no slot for a new one, and resolves to its
 * slot. A preview on the lamp ends; with `keepLit`, a new color stays lit.
 */
export function colorsSaveCustom(color: {
  slot: number | null;
  name: string;
  hex: string;
  keepLit: boolean;
}): Promise<number> {
  return invoke('colors_save_custom', color);
}

/** Names a built-in color, null for its default name. */
export function colorsRenameOriginal(slot: number, name: string | null): Promise<void> {
  return invoke('colors_rename_original', { slot, name });
}

export function colorsSetCycle(slots: number[]): Promise<void> {
  return invoke('colors_set_cycle', { slots });
}

/** Deletes a custom color, even in use: the backend applies the fallbacks. */
export function colorsDelete(slot: number): Promise<void> {
  return invoke('colors_delete', { slot });
}

export function colorsShareCode(): Promise<string> {
  return invoke('colors_share_code');
}

/** Writes the share code to the clipboard, which the app never reads. */
export function colorsCopyCode(): Promise<void> {
  return invoke('colors_copy_code');
}

/** The colors of a pasted text, null when it holds no readable code. */
export function colorsReadCode(text: string): Promise<Palette | null> {
  return invoke('colors_read_code', { text });
}

export function colorsImport(text: string): Promise<void> {
  return invoke('colors_import', { text });
}

/** Shows a code on the lamp while the editor changes it; null ends the preview. */
export function colorsPreview(hex: string | null): Promise<void> {
  return invoke('colors_preview', { hex });
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

/** Starts the update to the newest firmware 2.x; `firmware-status` events follow it. */
export function firmwareInstall(): Promise<void> {
  return invoke('firmware_install');
}

/** Starts the update to a version of the list, "X.Y.Z". */
export function firmwareInstallManual(version: string): Promise<void> {
  return invoke('firmware_install_manual', { version });
}

/** Stops an update that waits for the bootloader. */
export function firmwareCancel(): Promise<void> {
  return invoke('firmware_cancel');
}

export function openDiagnostics(): Promise<void> {
  return invoke('open_diagnostics');
}

/** The guided update, to the newest firmware 2.x. */
export function openLampUpdate(): Promise<void> {
  return invoke('open_lamp_update');
}

/** The reinstall of a version of the user's choice. */
export function openLampInstall(): Promise<void> {
  return invoke('open_lamp_install');
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
