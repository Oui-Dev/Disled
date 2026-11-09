/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Debug console commands (debug builds only). Type "help" in a serial
 * terminal opened on the Disled console port.
 */
#ifndef DISLED_APP_CONSOLE_H
#define DISLED_APP_CONSOLE_H

#if DISLED_DEBUG

#include <stdint.h>

void app_console_init(void);
void app_console_task(uint32_t now_ms);

#endif

#endif /* DISLED_APP_CONSOLE_H */
