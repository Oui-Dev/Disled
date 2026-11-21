// SPDX-License-Identifier: GPL-3.0-or-later
// SPDX-FileCopyrightText: 2026 The Disled contributors

import { describe, expect, it } from 'vitest';
import { compare } from './version';

describe('compare', () => {
  it('orders a version against the lamp, part by part, as numbers', () => {
    expect(compare('2.1.0', [2, 1, 0])).toBe(0);
    expect(compare('2.1.1', [2, 1, 0])).toBeGreaterThan(0);
    expect(compare('2.0.9', [2, 1, 0])).toBeLessThan(0);
    expect(compare('2.10.0', [2, 9, 9])).toBeGreaterThan(0);
    expect(compare('3.0.0', [2, 99, 99])).toBeGreaterThan(0);
  });
});
