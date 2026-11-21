// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import type { Capabilities } from '@/bindings/Capabilities';

/**
 * What a plugin may do besides running a program, in the order the windows
 * list it, each with its icon. Their texts are under the same keys.
 */
export const CAPABILITIES = [
  { key: 'color', icon: 'i-lucide-palette' },
  { key: 'alert', icon: 'i-lucide-bell' },
  { key: 'lamp', icon: 'i-lucide-eye' },
  { key: 'events', icon: 'i-lucide-webhook' },
] as const satisfies readonly { key: keyof Capabilities; icon: string }[];
