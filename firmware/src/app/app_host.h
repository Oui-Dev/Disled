/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Host side of the application: answers protocol commands, notifies every
 * state change and streams telemetry on request.
 */
#ifndef DISLED_APP_HOST_H
#define DISLED_APP_HOST_H

#include <stdint.h>

void app_host_init(void);
void app_host_task(uint32_t now_ms);

#endif /* DISLED_APP_HOST_H */
