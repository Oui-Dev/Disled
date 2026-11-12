/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Application entry points, called from main().
 */
#ifndef DISLED_APP_H
#define DISLED_APP_H

#include "bsp/bsp_fault.h"

/* Brings up the drivers and the lamp. `fault` is the record left by a
 * crash before the last reset, if any. */
void app_init(const fault_record_t *fault);

/* Main loop: runs the scheduler and feeds the watchdog. Never returns. */
__attribute__((noreturn)) void app_run(void);

#endif /* DISLED_APP_H */
