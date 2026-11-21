// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The plugins, as the integrations window shows them. The backend reads
// their folder, keeps their approvals and runs them (plugins/mod.rs); no
// event carries a token, which only the note of a plugin asks for. The
// catalog comes from the network, only when its tab opens
// (plugins/catalog.rs).

import { open } from '@tauri-apps/plugin-dialog';
import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { CatalogError } from '@/bindings/CatalogError';
import type { CatalogView } from '@/bindings/CatalogView';
import type { PluginView } from '@/bindings/PluginView';
import type { Reason } from '@/bindings/Reason';
import {
  pluginApprove,
  pluginInstall,
  pluginOpenSource,
  pluginRegenerateToken,
  pluginRemove,
  pluginSetColor,
  pluginSetEnabled,
  pluginsCatalog,
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

  /** The plugins of the catalog, none until it was read. */
  const catalog = ref<CatalogView[] | null>(null);
  /** The catalog is being read. */
  const catalogLoading = ref(false);
  /** Why the last reading or install of the catalog failed, until the next one. */
  const catalogError = ref<CatalogError | null>(null);
  /** The plugin of the catalog being installed. */
  const installing = ref<string | null>(null);
  /**
   * Counts the approvals and the new tokens. The fingerprint of a plugin
   * stays through both, so the note asks for its token again at each.
   */
  const tokenChanges = ref(0);

  /** Reads the catalog again, from the network. */
  async function loadCatalog(): Promise<void> {
    catalogLoading.value = true;
    try {
      catalog.value = await pluginsCatalog();
      catalogError.value = null;
    } catch (error) {
      catalogError.value = error as CatalogError;
    } finally {
      catalogLoading.value = false;
    }
  }

  /**
   * Installs a plugin of the catalog, whose id it returns for the question
   * that follows; a failure keeps its cause.
   */
  async function install(id: string): Promise<string | null> {
    installing.value = id;
    try {
      const installed = await pluginInstall(id);
      catalogError.value = null;
      await refresh();
      return installed;
    } catch (error) {
      catalogError.value = error as CatalogError;
      return null;
    } finally {
      installing.value = null;
    }
  }

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

  /** Runs a change that may give a plugin another token. */
  async function changeToken(action: () => Promise<void>): Promise<boolean> {
    const done = await change(action);
    if (done) {
      tokenChanges.value += 1;
    }
    return done;
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
    catalog,
    catalogLoading,
    catalogError,
    installing,
    tokenChanges,
    refresh,
    importPlugin,
    loadCatalog,
    install,
    openSource: (id: string) => change(() => pluginOpenSource(id)),
    approve: (id: string, fingerprint: string) => changeToken(() => pluginApprove(id, fingerprint)),
    setEnabled: (id: string, enabled: boolean) => change(() => pluginSetEnabled(id, enabled)),
    setColor: (id: string, state: string, slot: number) =>
      change(() => pluginSetColor(id, state, slot)),
    regenerateToken: (id: string) => changeToken(() => pluginRegenerateToken(id)),
    remove: (id: string) => change(() => pluginRemove(id)),
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
