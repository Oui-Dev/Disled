// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import ui from '@nuxt/ui/vue-plugin';
import { createPinia } from 'pinia';
import { createApp } from 'vue';
import App from './App.vue';
import { i18n } from './i18n';
import { useSettingsStore } from './stores/settings';
import './assets/css/main.css';

const app = createApp(App).use(createPinia()).use(i18n).use(ui);
// Settings first, so that the window opens in its language and color mode.
await useSettingsStore().start();
app.mount('#app');
