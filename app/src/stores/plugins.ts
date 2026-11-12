// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The plugins, as the integrations window shows them. The backend reads
// their folder, keeps their approvals and runs them (plugins/mod.rs); no
// event carries a token, which only the note of a plugin asks for.

import { open } from '@tauri-apps/plugin-dialog';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { PluginView } from '@/bindings/PluginView';
import type { Reason } from '@/bindings/Reason';
import {
  pluginApprove,
  pluginCopyNote,
  pluginRegenerateToken,
  pluginRemove,
  pluginSetColor,
  pluginSetEnabled,
  pluginsImport,
  pluginsList,
  pluginsOpenFolder,
} from '@/ipc/commands';
import { follow } from '@/ipc/events';
import { useErrorStore } from './errors';

export const usePluginsStore = defineStore('plugins', () => {
  const errors = useErrorStore();

  const plugins = ref<PluginView[]>([]);
  /** Why the last import was refused, until the next one. */
  const importRefused = ref<Reason | null>(null);

  /** Runs a change; a failure becomes the last error, a success clears it. */
  async function change(action: () => Promise<void>): Promise<boolean> {
    try {
      await action();
      errors.clear();
      return true;
    } catch (error) {
      errors.reportFailure(error);
      return false;
    }
  }

  /** Reads the folder again, as when the window gets the focus. */
  async function refresh(): Promise<void> {
    plugins.value = await pluginsList();
  }

  /**
   * Picks a zip file and imports its plugin, whose id it returns for the
   * question that follows; a refusal keeps its reason.
   */
  async function importPlugin(): Promise<string | null> {
    const path = await open({
      multiple: false,
      directory: false,
      filters: [{ name: 'Plugin', extensions: ['zip'] }],
    });
    if (typeof path !== 'string') {
      return null;
    }
    try {
      const id = await pluginsImport(path);
      importRefused.value = null;
      return id;
    } catch (reason) {
      importRefused.value = reason as Reason;
      return null;
    }
  }

  return {
    plugins,
    importRefused,
    refresh,
    importPlugin,
    approve: (id: string, fingerprint: string) => change(() => pluginApprove(id, fingerprint)),
    setEnabled: (id: string, enabled: boolean) => change(() => pluginSetEnabled(id, enabled)),
    setColor: (id: string, state: string, slot: number) =>
      change(() => pluginSetColor(id, state, slot)),
    regenerateToken: (id: string) => change(() => pluginRegenerateToken(id)),
    remove: (id: string) => change(() => pluginRemove(id)),
    copyNote: (id: string) => change(() => pluginCopyNote(id)),
    openFolder: () => change(pluginsOpenFolder),
    /** Follows the backend events. Call once per window. */
    start: () =>
      follow(
        'plugins-changed',
        (next) => {
          plugins.value = next;
        },
        pluginsList,
      ),
  };
});
