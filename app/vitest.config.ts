// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { fileURLToPath } from 'node:url';
import { defineConfig } from 'vitest/config';

// The unit tests run the logic of the windows in Node, without the plugins
// of the build: the modules they test import no component, and the tests
// stand in for the backend where they need one.
export default defineConfig({
  resolve: {
    alias: { '@': fileURLToPath(new URL('./src', import.meta.url)) },
  },
  test: {
    include: ['src/**/*.test.ts'],
  },
});
