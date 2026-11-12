/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Cooperative scheduler: tasks run to completion from the main loop. A task
 * of period 0 runs on every pass, and a late task runs once, without a burst
 * to catch up.
 */
#ifndef DISLED_SCHEDULER_H
#define DISLED_SCHEDULER_H

#include <stddef.h>
#include <stdint.h>

typedef struct {
    uint32_t period_ms;
    void (*run)(uint32_t now_ms);
    uint32_t next_ms; /* managed by the scheduler */
} scheduler_task_t;

/* Makes every task due at `now_ms`. */
void scheduler_start(scheduler_task_t *tasks, size_t count, uint32_t now_ms);

/* Runs the due tasks in table order. */
void scheduler_run(scheduler_task_t *tasks, size_t count, uint32_t now_ms);

#endif /* DISLED_SCHEDULER_H */
