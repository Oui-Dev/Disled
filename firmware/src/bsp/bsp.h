/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Board start-up: clocks, safe pin states, reset cause, time base.
 */
#ifndef DISLED_BSP_H
#define DISLED_BSP_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"

/* Call first in main(). The LED pins keep their reset state (inputs, the
 * gate pull-downs hold the LED off) until the LED driver takes them. */
void bsp_init(void);

/* Why the chip last reset, captured by bsp_init(). */
reset_cause_t bsp_reset_cause(void);

/* False if the crystal did not start: the chip then runs on its internal
 * oscillator, too inaccurate for USB. */
bool bsp_crystal_ok(void);

/* Clock of the timers on APB1 (TIM2..TIM4), in Hz. */
uint32_t bsp_apb1_timer_clock_hz(void);

/* Stack use: fills the free stack with a pattern (first thing in main),
 * then reports how many bytes were never touched since (the smallest
 * margin seen). */
void bsp_stack_paint(void);
uint16_t bsp_stack_free_min(void);

__attribute__((noreturn)) void bsp_system_reset(void);

#endif /* DISLED_BSP_H */
