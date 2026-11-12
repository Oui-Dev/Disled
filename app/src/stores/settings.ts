// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// Settings saved on this computer by the backend, which applies them to the lamp at
// every connection and to the app. Every window keeps a copy, refreshed by
// `settings-changed`.

import { defineStore } from 'pinia';
import { ref } from 'vue';
import type { Locale } from '@/bindings/Locale';
import type { Settings } from '@/bindings/Settings';
import type { SettingsPatch } from '@/bindings/SettingsPatch';
import { autostartGet, autostartSet, localeGet, settingsGet, settingsSet } from '@/ipc/commands';
import { follow } from '@/ipc/events';
import { useErrorStore } from './errors';

export const useSettingsStore = defineStore('settings', () => {
  const errors = useErrorStore();

  /** Null until the backend answers. */
  const settings = ref<Settings | null>(null);
  /** The language in force: the setting, or the system's when it says so. */
  const locale = ref<Locale>('en');
  const autostart = ref(false);
  /** Counts the `settings-changed` events, to tell when one comes during an update. */
  let changes = 0;

  async function update(patch: SettingsPatch): Promise<void> {
    try {
      const before = changes;
      const next = await settingsSet(patch);
      // An event heard meanwhile may come from a later update, and the
      // events keep the settings current anyway.
      if (changes === before) {
        settings.value = next;
      }
      errors.clear();
    } catch (error) {
      errors.reportFailure(error);
    }
  }

  async function setAutostart(enabled: boolean): Promise<void> {
    try {
      autostart.value = await autostartSet(enabled);
    } catch (error) {
      errors.reportFailure(error);
      autostart.value = await autostartGet();
    }
  }

  /** Follows the backend events. Call once per window, before showing it. */
  async function start(): Promise<void> {
    await Promise.all([
      follow(
        'settings-changed',
        (next) => {
          changes += 1;
          settings.value = next;
        },
        settingsGet,
      ),
      follow(
        'locale-changed',
        (next) => {
          locale.value = next;
        },
        localeGet,
      ),
      autostartGet().then((enabled) => {
        autostart.value = enabled;
      }),
    ]);
  }

  return { settings, locale, autostart, update, setAutostart, start };
});
