<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The popup under the tray icon: everyday control of the lamp, and a
  settings page. The window takes the height of its content; Escape hides
  it, and it opens again on the lamp page.
-->
<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { onKeyStroke } from '@vueuse/core';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import { useErrorText } from '@/composables/useErrorText';
import { usePopupAutoSize } from '@/composables/usePopupAutoSize';
import { useUpdateStatus } from '@/composables/useUpdateStatus';
import { popupHide, quit } from '@/ipc/commands';
import { useErrorStore } from '@/stores/errors';
import { useLampStore } from '@/stores/lamp';

type Page = 'lamp' | 'settings';

const { t } = useI18n();
const lamp = useLampStore();
const errors = useErrorStore();
const errorText = useErrorText();
const { offerLine } = useUpdateStatus();

// The page, then the footer. The window fits both, and the footer stays at
// the bottom of a window taller than the content.
const { fit } = usePopupAutoSize(
  useTemplateRef<HTMLElement>('body'),
  useTemplateRef<HTMLElement>('footer'),
);

const page = ref<Page>('lamp');
const pageHeading = useTemplateRef<HTMLElement>('pageHeading');

/** The update on offer, announced by the settings button of the lamp page. */
const settingsOffer = computed(() => (page.value === 'lamp' ? offerLine.value : null));

/** Moves the focus to the title of the new page, for keyboards and screen readers. */
async function show(next: Page): Promise<void> {
  page.value = next;
  await nextTick();
  pageHeading.value?.focus();
}

/** Back to the lamp page once hidden, sized for it before the popup shows again. */
async function reset(): Promise<void> {
  if (page.value === 'lamp') {
    return;
  }
  page.value = 'lamp';
  await nextTick();
  await fit();
}

onKeyStroke('Escape', async () => {
  // An open menu, still in the page while this runs, closes first and the
  // popup stays.
  if (document.querySelector('[role="menu"]')) {
    return;
  }
  await popupHide();
  await reset();
});

// The backend hides the popup when it loses the focus.
let stopFollowingFocus: UnlistenFn | undefined;
onMounted(async () => {
  stopFollowingFocus = await getCurrentWebviewWindow().onFocusChanged(({ payload: focused }) => {
    if (!focused) {
      void reset();
    }
  });
});
onBeforeUnmount(() => stopFollowingFocus?.());

const status = computed(() => {
  if (!lamp.connection.connected) {
    return t('status.noLamp');
  }
  if (lamp.generation === 'v1') {
    return t('status.needsUpdate');
  }
  // A lamp newer than the app is connected, and its alert says the rest.
  return lamp.state || lamp.generation === 'newer' ? t('status.connected') : t('status.connecting');
});
</script>

<template>
  <main class="flex min-h-screen flex-col">
    <div ref="body" class="flex flex-col gap-4 p-4 pb-3">
      <header>
        <h1 class="text-base font-bold tracking-wide text-highlighted">Disled</h1>
        <p role="status" class="text-xs text-muted">{{ status }}</p>
      </header>

      <section v-if="page === 'lamp'" class="flex flex-col gap-4">
        <h2 ref="pageHeading" tabindex="-1" class="sr-only">{{ t('popup.lampPage') }}</h2>
        <p v-if="!lamp.connection.connected" class="text-xs text-muted">
          {{ t('lamp.unplugged') }}
        </p>
        <LampV1Warning />
        <LampColorPicker />
        <LampBrightness />
      </section>

      <section v-else class="flex flex-col gap-3">
        <div class="flex items-center justify-between gap-2">
          <h2
            ref="pageHeading"
            tabindex="-1"
            class="text-sm font-semibold text-highlighted focus:outline-none"
          >
            {{ t('common.settings') }}
          </h2>
          <PopupMenu />
        </div>
        <LampV1Warning />
        <PopupUpdates />
        <PopupSettings />
      </section>

      <!-- UAlert has no role of its own, so this one gets alert for screen readers. -->
      <UAlert
        v-if="errors.last"
        role="alert"
        color="error"
        variant="subtle"
        icon="i-lucide-triangle-alert"
        :description="errorText(errors.last)"
        :close="{ size: 'sm' }"
        :ui="{ root: 'gap-2 p-3', icon: 'size-4', description: 'text-xs wrap-break-word' }"
        @update:open="errors.clear()"
      />
    </div>

    <footer ref="footer" class="mt-auto flex items-center justify-between gap-2 px-4 pb-3">
      <!-- An update on offer, or a lamp to update, turns it primary, with a dot, and its tooltip says which. -->
      <UButton
        :color="settingsOffer ? 'primary' : 'neutral'"
        variant="link"
        size="sm"
        class="px-0"
        :title="settingsOffer ?? undefined"
        @click="show(page === 'lamp' ? 'settings' : 'lamp')"
      >
        <template #leading="{ ui }">
          <UChip as="span" :show="settingsOffer !== null" size="sm">
            <UIcon
              :name="page === 'lamp' ? 'i-lucide-settings' : 'i-lucide-arrow-left'"
              :class="ui.leadingIcon()"
            />
          </UChip>
        </template>
        {{ page === 'lamp' ? t('common.settings') : t('common.back') }}
      </UButton>
      <UButton
        color="neutral"
        variant="link"
        size="sm"
        class="px-0"
        icon="i-lucide-power"
        @click="quit"
      >
        {{ t('common.quit') }}
      </UButton>
    </footer>
  </main>
</template>
