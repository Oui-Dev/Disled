/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <unity.h>

#include "power_limiter.h"

/* Worst-case full-scale currents (constraint N2 in hardware/README.md). */
static const power_config_t config = {
    .full_scale_ma = { 86, 63, 125 },
    .unconfigured_cap_ma = 70,
    .configured_cap_ma = 125,
};

static const rgb16_t white = { 65535, 65535, 65535 };

void setUp(void)
{
}

void tearDown(void)
{
}

static void test_cap_depends_on_the_usb_state(void)
{
    TEST_ASSERT_EQUAL_UINT16(70, power_cap_ma(&config, USB_LINK_UNCONFIGURED));
    TEST_ASSERT_EQUAL_UINT16(125, power_cap_ma(&config, USB_LINK_CONFIGURED));
    TEST_ASSERT_EQUAL_UINT16(0, power_cap_ma(&config, USB_LINK_SUSPENDED));
}

static void test_estimate_is_duty_times_full_scale(void)
{
    const rgb16_t red = { 65535, 0, 0 };
    TEST_ASSERT_EQUAL_UINT32(860, power_estimate_ma_x10(&config, red));
    TEST_ASSERT_EQUAL_UINT32(2740, power_estimate_ma_x10(&config, white));
    const rgb16_t half_blue = { 0, 0, 32768 };
    TEST_ASSERT_UINT32_WITHIN(1, 625, power_estimate_ma_x10(&config, half_blue));
}

static void test_light_under_the_cap_is_unchanged(void)
{
    const rgb16_t blue = { 0, 0, 65535 };
    const rgb16_t out = power_limit(&config, blue, 125);
    TEST_ASSERT_EQUAL_MEMORY(&blue, &out, sizeof blue);
}

static void test_white_is_scaled_to_the_cap_keeping_the_hue(void)
{
    const rgb16_t out = power_limit(&config, white, 125);
    TEST_ASSERT_TRUE(power_estimate_ma_x10(&config, out) <= 1250u);
    TEST_ASSERT_TRUE(power_estimate_ma_x10(&config, out) >= 1245u);
    TEST_ASSERT_UINT16_WITHIN(1, out.r, out.g);
    TEST_ASSERT_UINT16_WITHIN(1, out.r, out.b);
}

static void test_unconfigured_cap(void)
{
    const rgb16_t out = power_limit(&config, white, power_cap_ma(&config, USB_LINK_UNCONFIGURED));
    TEST_ASSERT_TRUE(power_estimate_ma_x10(&config, out) <= 700u);
}

static void test_zero_cap_turns_everything_off(void)
{
    const rgb16_t out = power_limit(&config, white, 0);
    TEST_ASSERT_EQUAL_UINT16(0, out.r);
    TEST_ASSERT_EQUAL_UINT16(0, out.g);
    TEST_ASSERT_EQUAL_UINT16(0, out.b);
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_cap_depends_on_the_usb_state);
    RUN_TEST(test_estimate_is_duty_times_full_scale);
    RUN_TEST(test_light_under_the_cap_is_unchanged);
    RUN_TEST(test_white_is_scaled_to_the_cap_keeping_the_hue);
    RUN_TEST(test_unconfigured_cap);
    RUN_TEST(test_zero_cap_turns_everything_off);
    return UNITY_END();
}
