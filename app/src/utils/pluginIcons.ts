// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

// The icons a plugin may name (src-tauri/src/plugins/manifest.rs), written
// out in full: the bundle holds only the icons its code names, and the app
// never loads one from the Internet.

const ICONS: Record<string, string> = {
  terminal: 'i-lucide-terminal',
  timer: 'i-lucide-timer',
  bot: 'i-lucide-bot',
  code: 'i-lucide-code',
  'git-branch': 'i-lucide-git-branch',
  calendar: 'i-lucide-calendar',
  bell: 'i-lucide-bell',
  mail: 'i-lucide-mail',
  'message-square': 'i-lucide-message-square',
  music: 'i-lucide-music',
  headphones: 'i-lucide-headphones',
  coffee: 'i-lucide-coffee',
  zap: 'i-lucide-zap',
  puzzle: 'i-lucide-puzzle',
};

/** The icon of a plugin, the puzzle for one outside the list. */
export function pluginIcon(name: string | null | undefined): string {
  return (name && ICONS[name]) || 'i-lucide-puzzle';
}
