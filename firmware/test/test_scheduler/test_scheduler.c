/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <unity.h>

#include "scheduler.h"

static unsigned fast_runs;
static unsigned slow_runs;
static unsigned always_runs;

static void fast(uint32_t now_ms)
{
    (void)now_ms;
    fast_runs++;
}

static void slow(uint32_t now_ms)
{
    (void)now_ms;
    slow_runs++;
}

static void always(uint32_t now_ms)
{
    (void)now_ms;
    always_runs++;
}

static scheduler_task_t tasks[] = {
    { 10, fast, 0 },
    { 100, slow, 0 },
    { 0, always, 0 },
};
#define TASK_COUNT (sizeof tasks / sizeof tasks[0])

void setUp(void)
{
    fast_runs = 0;
    slow_runs = 0;
    always_runs = 0;
}

void tearDown(void)
{
}

static void test_tasks_run_at_their_period(void)
{
    scheduler_start(tasks, TASK_COUNT, 1000);
    for (uint32_t t = 1000; t < 2000; t++) {
        scheduler_run(tasks, TASK_COUNT, t);
    }
    TEST_ASSERT_EQUAL_UINT(100, fast_runs);
    TEST_ASSERT_EQUAL_UINT(10, slow_runs);
    TEST_ASSERT_EQUAL_UINT(1000, always_runs);
}

static void test_a_late_task_runs_once_without_burst(void)
{
    scheduler_start(tasks, TASK_COUNT, 0);
    scheduler_run(tasks, TASK_COUNT, 0);
    TEST_ASSERT_EQUAL_UINT(1, fast_runs);

    /* The loop stalled for 55 ms. */
    scheduler_run(tasks, TASK_COUNT, 55);
    TEST_ASSERT_EQUAL_UINT(2, fast_runs);
    scheduler_run(tasks, TASK_COUNT, 56);
    TEST_ASSERT_EQUAL_UINT(2, fast_runs);
    scheduler_run(tasks, TASK_COUNT, 65);
    TEST_ASSERT_EQUAL_UINT(3, fast_runs);
}

static void test_survives_time_wrap_around(void)
{
    const uint32_t start = 0xFFFFFFF0u;
    scheduler_start(tasks, TASK_COUNT, start);
    for (uint32_t i = 0; i < 100; i++) {
        scheduler_run(tasks, TASK_COUNT, start + i);
    }
    TEST_ASSERT_EQUAL_UINT(10, fast_runs);
    TEST_ASSERT_EQUAL_UINT(1, slow_runs);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_tasks_run_at_their_period);
    RUN_TEST(test_a_late_task_runs_once_without_burst);
    RUN_TEST(test_survives_time_wrap_around);
    return UNITY_END();
}
