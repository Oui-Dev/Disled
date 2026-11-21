// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { computed } from 'vue';
import { useFirmwareStore } from '@/stores/firmware';
import { stepOf } from '@/utils/flash';

/** Where the lamp update stands, as the update windows show it. */
export function useFlashSteps() {
  const firmware = useFirmwareStore();

  const flash = computed(() => firmware.status.flash);
  /** No update runs, and none failed: the window offers one. */
  const idle = computed(() => flash.value.step === 'idle');
  const failed = computed(() => flash.value.step === 'failed');
  const finished = computed(() => flash.value.step === 'done' || failed.value);
  const waitingForBootloader = computed(() => stepOf(flash.value) === 1);

  return { flash, idle, failed, finished, waitingForBootloader };
}
