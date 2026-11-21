// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { beforeEach, describe, expect, it, vi } from 'vitest';
import { follow } from './events';

// The window, whose listeners the tests call as the backend would.
const listeners = vi.hoisted(() => new Map<string, (event: { payload: unknown }) => void>());
vi.mock('@tauri-apps/api/webviewWindow', () => ({
  getCurrentWebviewWindow: () => ({
    listen: async (event: string, listener: (event: { payload: unknown }) => void) => {
      listeners.set(event, listener);
      return () => listeners.delete(event);
    },
  }),
}));

/** The backend sends an event to the window. */
function send(event: string, payload: unknown): void {
  listeners.get(event)?.({ payload });
}

describe('follow', () => {
  beforeEach(() => listeners.clear());

  it('starts from the answer of the read, then follows the events', async () => {
    const seen: string[] = [];
    await follow(
      'locale-changed',
      (locale) => seen.push(locale),
      async () => 'fr',
    );
    send('locale-changed', 'en');
    expect(seen).toEqual(['fr', 'en']);
  });

  it('drops an answer older than an event heard while it came', async () => {
    const seen: string[] = [];
    await follow(
      'locale-changed',
      (locale) => seen.push(locale),
      async () => {
        send('locale-changed', 'fr');
        return 'en';
      },
    );
    expect(seen).toEqual(['fr']);
  });
});
