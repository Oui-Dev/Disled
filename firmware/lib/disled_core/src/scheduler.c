/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "scheduler.h"

#include "time_utils.h"

void scheduler_start(scheduler_task_t *tasks, size_t count, uint32_t now_ms)
{
    for (size_t i = 0; i < count; i++) {
        tasks[i].next_ms = now_ms;
    }
}

void scheduler_run(scheduler_task_t *tasks, size_t count, uint32_t now_ms)
{
    for (size_t i = 0; i < count; i++) {
        scheduler_task_t *task = &tasks[i];
        if (task->period_ms != 0u && !time_reached(now_ms, task->next_ms)) {
            continue;
        }
        task->run(now_ms);
        if (task->period_ms != 0u) {
            task->next_ms += task->period_ms;
            /* Fell behind by more than a period: resynchronize. */
            if (time_reached(now_ms, task->next_ms)) {
                task->next_ms = now_ms + task->period_ms;
            }
        }
    }
}
