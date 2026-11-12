/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <unity.h>

#include "fmt.h"

static char out[64];

void setUp(void)
{
}

void tearDown(void)
{
}

static void test_integers(void)
{
    fmt_snprintf(out, sizeof out, "%d %d %u", -42, 7, 4000000000u);
    TEST_ASSERT_EQUAL_STRING("-42 7 4000000000", out);
    fmt_snprintf(out, sizeof out, "%ld %lu", -2147483647L - 1L, 4294967295UL);
    TEST_ASSERT_EQUAL_STRING("-2147483648 4294967295", out);
}

static void test_padding(void)
{
    fmt_snprintf(out, sizeof out, "%08lu %4d|%-4d|", 1234ul, 42, 42);
    TEST_ASSERT_EQUAL_STRING("00001234   42|42  |", out);
    fmt_snprintf(out, sizeof out, "%05d|%-5d|", -42, -42);
    TEST_ASSERT_EQUAL_STRING("-0042|-42  |", out);
}

static void test_strings(void)
{
    fmt_snprintf(out, sizeof out, "[%s] [%5s] [%-5s] %%", "ab", "ab", "ab");
    TEST_ASSERT_EQUAL_STRING("[ab] [   ab] [ab   ] %", out);
}

static void test_truncation_reports_the_full_length(void)
{
    char small[6];
    const size_t length = fmt_snprintf(small, sizeof small, "%s", "abcdefgh");
    TEST_ASSERT_EQUAL_size_t(8, length);
    TEST_ASSERT_EQUAL_STRING("abcde", small);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_integers);
    RUN_TEST(test_padding);
    RUN_TEST(test_strings);
    RUN_TEST(test_truncation_reports_the_full_length);
    return UNITY_END();
}
