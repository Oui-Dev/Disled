<!--
  SPDX-License-Identifier: GPL-3.0-or-later
  SPDX-FileCopyrightText: 2026 The Disled contributors

  The popup under the tray icon: everyday control of the lamp, the settings,
  and the pages of the colors (library, editor, share, import). The window
  takes the height of its content; Escape hides it, and it opens again on
  the lamp page, with no preview left on the lamp and no draft kept.
-->
<script setup lang="ts">
import type { UnlistenFn } from '@tauri-apps/api/event';
import { getCurrentWebviewWindow } from '@tauri-apps/api/webviewWindow';
import { onKeyStroke } from '@vueuse/core';
import { computed, nextTick, onBeforeUnmount, onMounted, ref, useTemplateRef } from 'vue';
import { useI18n } from 'vue-i18n';
import type { Palette } from '@/bindings/Palette';
import { useErrorText } from '@/composables/useErrorText';
import { usePopupAutoSize } from '@/composables/usePopupAutoSize';
import { useUpdateStatus } from '@/composables/useUpdateStatus';
import { popupHide, quit } from '@/ipc/commands';
import { useErrorStore } from '@/stores/errors';
import { useLampStore } from '@/stores/lamp';
import { BUILT_IN_COLORS } from '@/utils/lamp';

type Page = 'lamp' | 'settings' | 'colors' | 'editor' | 'share' | 'import';

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

/** The color in the editor, null for a new one, and the page it came from. */
const editing = ref<{ slot: number | null; from: 'lamp' | 'colors' }>({ slot: null, from: 'lamp' });
/** The code being imported, and what it holds. */
const importing = ref<{ text: string; palette: Palette } | null>(null);

const editor = useTemplateRef<{ save(): Promise<void>; canSave: boolean }>('editor');
const importer = useTemplateRef<{ apply(): Promise<void> }>('importer');

/** The update on offer, announced by the settings button of the lamp page. */
const settingsOffer = computed(() => (page.value === 'lamp' ? offerLine.value : null));

/**
 * Moves to a page and its focus, for keyboards and screen readers: the title
 * of the page, or the name of the color in the editor, which focuses itself.
 */
async function show(next: Page): Promise<void> {
  page.value = next;
  await nextTick();
  if (next !== 'editor') {
    document.querySelector<HTMLElement>('[data-page-heading]')?.focus();
  }
}

function edit(slot: number | null, from: 'lamp' | 'colors'): void {
  editing.value = { slot, from };
  void show('editor');
}

function receive(text: string, palette: Palette): void {
  importing.value = { text, palette };
  void show('import');
}

/**
 * Back to the lamp page once hidden, sized for it before the popup shows
 * again. Leaving the editor ends its preview on the lamp.
 */
async function reset(): Promise<void> {
  importing.value = null;
  if (page.value === 'lamp') {
    return;
  }
  page.value = 'lamp';
  await nextTick();
  await fit();
}

onKeyStroke('Escape', async () => {
  // An open menu, popover or list, still in the page while this runs,
  // closes first and the popup stays.
  if (document.querySelector('[role="menu"], [role="dialog"], [role="listbox"]')) {
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

/** Where the left link of the footer leads, on the pages that have one. */
const back = computed<Page | null>(() => {
  switch (page.value) {
    case 'lamp':
      return 'settings';
    case 'settings':
    case 'colors':
      return 'lamp';
    case 'share':
      return 'colors';
    default:
      return null;
  }
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
        <h2 data-page-heading tabindex="-1" class="sr-only">{{ t('popup.lampPage') }}</h2>
        <p v-if="!lamp.connection.connected" class="text-xs text-muted">
          {{ t('lamp.unplugged') }}
        </p>
        <LampV1Warning />
        <LampColorPicker @manage="show('colors')" @create="edit(null, 'lamp')" />
        <LampBrightness />
      </section>

      <section v-else-if="page === 'settings'" class="flex flex-col gap-3">
        <div class="flex items-center justify-between gap-2">
          <h2
            data-page-heading
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

      <ColorsLibrary
        v-else-if="page === 'colors'"
        @edit="(slot: number) => edit(slot, 'colors')"
        @create="edit(null, 'colors')"
        @share="show('share')"
      />

      <template v-else-if="page === 'editor'">
        <ColorsOriginalEditor
          v-if="editing.slot !== null && editing.slot < BUILT_IN_COLORS.length"
          ref="editor"
          :slot="editing.slot"
          @done="show(editing.from)"
        />
        <ColorsEditor
          v-else
          ref="editor"
          :slot="editing.slot"
          :from-popover="editing.from === 'lamp'"
          @done="show(editing.from)"
        />
      </template>

      <ColorsShare v-else-if="page === 'share'" @received="receive" />

      <ColorsImport
        v-else-if="page === 'import' && importing"
        ref="importer"
        :text="importing.text"
        :palette="importing.palette"
        @done="show('colors')"
      />

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
      <template v-if="page === 'editor'">
        <UButton color="neutral" variant="link" size="sm" class="px-0" @click="show(editing.from)">
          {{ t('common.cancel') }}
        </UButton>
        <UButton size="sm" :disabled="!editor?.canSave" @click="editor?.save()">
          {{ t('common.save') }}
        </UButton>
      </template>
      <template v-else-if="page === 'import'">
        <UButton color="neutral" variant="link" size="sm" class="px-0" @click="show('share')">
          {{ t('common.cancel') }}
        </UButton>
        <UButton size="sm" @click="importer?.apply()">{{ t('colors.import.action') }}</UButton>
      </template>
      <template v-else>
        <!-- An update on offer, or a lamp to update, turns it primary, with a dot, and its tooltip says which. -->
        <UButton
          :color="settingsOffer ? 'primary' : 'neutral'"
          variant="link"
          size="sm"
          class="px-0"
          :title="settingsOffer ?? undefined"
          @click="back && show(back)"
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
      </template>
    </footer>
  </main>
</template>
