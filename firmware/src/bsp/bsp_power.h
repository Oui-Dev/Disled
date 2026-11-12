/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Power saving for USB suspend: the internal 8 MHz oscillator divided by 8,
 * crystal and PLL off. The core never sleeps, since it never wakes up from a
 * WFI in the main loop, which keeps feeding the watchdog, slowly.
 */
#ifndef DISLED_BSP_POWER_H
#define DISLED_BSP_POWER_H

#include <stdbool.h>

/* Switches the core to 1 MHz (internal oscillator / 8) and stops the
 * crystal and the PLL. USB, which needs the PLL, must be suspended. */
void bsp_power_slow_clock(void);

/* Back to the crystal and the PLL (144 MHz), or their first start after
 * reset (bsp_init). Returns false if the core does not run from them: the
 * crystal did not start, and the chip stays on the internal oscillator
 * (8 MHz). Safe to call when already fast. The _from_isr variant expects
 * interrupts to be off, and does not retry a failed start before the next
 * slow clock; bsp_power_full_clock() retries it. */
bool bsp_power_full_clock(void);
bool bsp_power_full_clock_from_isr(void);

#endif /* DISLED_BSP_POWER_H */
