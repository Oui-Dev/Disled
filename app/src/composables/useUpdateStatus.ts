// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { computed } from 'vue';
import { useI18n } from 'vue-i18n';
import { useFirmwareStore } from '@/stores/firmware';
import { useLampStore } from '@/stores/lamp';
import { useUpdatesStore } from '@/stores/updates';

/** Where the app and lamp updates stand, worded for the popup. */
export function useUpdateStatus() {
  const { t, d, n } = useI18n();
  const updates = useUpdatesStore();
  const firmware = useFirmwareStore();
  const lamp = useLampStore();

  const installing = computed(() => updates.status.progress !== null);
  const checking = computed(() => updates.status.checking || firmware.status.checking);
  /** The update to install first: the app, since a new firmware may need a new app. */
  const offer = computed(() =>
    updates.status.available ? 'app' : firmware.updateForLamp ? 'lamp' : null,
  );

  /** The app: being installed, checked, failed, available or up to date; null before a check. */
  const appLine = computed(() => {
    const { available, progress, error, checked_at } = updates.status;
    if (installing.value) {
      return t('updates.downloading', {
        version: available,
        progress: n((progress ?? 0) / 100, 'percent'),
      });
    }
    // An update on offer keeps its line, the text of the popup alert, while
    // a new check runs.
    if (checking.value && !available) {
      return t('updates.checking');
    }
    if (error) {
      return t('updates.error', { detail: error });
    }
    if (available) {
      return t('updates.available', { version: available });
    }
    if (checked_at) {
      return t('updates.upToDate', { date: d(new Date(checked_at * 1000), 'short') });
    }
    return null;
  });

  /**
   * The lamp: a version 1 lamp to update, a failed check or a newer
   * firmware, after the app's; null otherwise.
   */
  const lampLine = computed(() => {
    if (lamp.generation === 'v1') {
      return t('lampV1.line');
    }
    const { checking: lampChecking, error } = firmware.status;
    const version = firmware.updateForLamp;
    // A lamp update on offer keeps its line too.
    if (lampChecking && !version) {
      return null;
    }
    if (error) {
      return t('updates.lampError', { detail: error });
    }
    if (!version) {
      return null;
    }
    const app = updates.status.available;
    return app
      ? t('updates.lampAfterApp', { version, app })
      : t('updates.lampAvailable', { version });
  });

  /**
   * The offer in a few words, or the lamp to update, for the tooltip of the
   * settings button in the popup footer.
   */
  const offerLine = computed(() => {
    if (lamp.generation === 'v1') {
      return t('lampV1.line');
    }
    if (offer.value === 'app') {
      return t('updates.toApp', { version: updates.status.available });
    }
    return offer.value === 'lamp' ? t('updates.toLamp') : null;
  });

  function checkAll(): Promise<unknown> {
    return Promise.all([updates.check(), firmware.check()]);
  }

  return { installing, checking, offer, appLine, lampLine, offerLine, checkAll };
}
