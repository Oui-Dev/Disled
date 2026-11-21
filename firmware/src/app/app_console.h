/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Debug console commands (debug builds only). Type "help" in a serial
 * terminal opened on the Disled console port.
 */
#ifndef DISLED_APP_CONSOLE_H
#define DISLED_APP_CONSOLE_H

#if DISLED_DEBUG

#include <stdint.h>

#include "disled_types.h"

void app_console_init(void);
void app_console_task(uint32_t now_ms);

/* Calibration: while "raw" is on, its duties, in sub-ticks, replace
 * `duty`, scaled down alike to the current cap of the link. */
void app_console_raw_output(uint16_t duty[LED_CHANNEL_COUNT]);

#endif

#endif /* DISLED_APP_CONSOLE_H */
