/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Millisecond time base (SysTick).
 */
#ifndef DISLED_BSP_TIME_H
#define DISLED_BSP_TIME_H

#include <stdint.h>

void bsp_time_init(void);

/* Milliseconds since boot. Wraps after about 49.7 days: compare times with
 * time_elapsed_ms() from time_utils.h, never with < or >. */
uint32_t bsp_time_ms(void);

/* The core clock changed (start of the crystal, USB suspend): restarts the
 * 1 ms period at the new SystemCoreClock. The time never goes back, but may
 * jump a millisecond ahead. */
void bsp_time_core_clock_changed(void);

/* Busy-waits at least the given number of microseconds (short waits: the
 * watchdog runs). */
void bsp_time_delay_us(uint32_t us);

/* Function called from the SysTick interrupt every millisecond, after the
 * time base is updated. One hook at most; NULL removes it. Keep it short. */
typedef void (*bsp_tick_hook_t)(void);
void bsp_time_set_tick_hook(bsp_tick_hook_t hook);

#endif /* DISLED_BSP_TIME_H */
