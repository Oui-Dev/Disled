/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * LED current budget: the cap in force and the current of duties. led_output
 * applies them (test_led_output), whose random test takes this cap as its
 * oracle.
 */
#include <unity.h>

#include "power_limiter.h"

/* Worst-case full-scale currents (constraint N2 in hardware/README.md). */
static const power_config_t config = {
    .full_scale_ma = { 86, 63, 125 },
    .unconfigured_cap_ma = 70,
    .configured_cap_ma = 125,
};

/* 100 % duty in sub-ticks, that of the lamp's PWM period. */
#define FULL 57600u

void setUp(void)
{
}

void tearDown(void)
{
}

static void test_cap_depends_on_the_usb_state(void)
{
    TEST_ASSERT_EQUAL_UINT16(70, power_cap_ma(&config, USB_LINK_UNCONFIGURED, 0));
    TEST_ASSERT_EQUAL_UINT16(125, power_cap_ma(&config, USB_LINK_CONFIGURED, 0));
    TEST_ASSERT_EQUAL_UINT16(0, power_cap_ma(&config, USB_LINK_SUSPENDED, 0));
}

static void test_an_unknown_usb_state_allows_nothing(void)
{
    TEST_ASSERT_EQUAL_UINT16(0, power_cap_ma(&config, (usb_link_t)3, 0));
}

static void test_an_override_replaces_the_cap_of_the_link(void)
{
    TEST_ASSERT_EQUAL_UINT16(300, power_cap_ma(&config, USB_LINK_CONFIGURED, 300));
    TEST_ASSERT_EQUAL_UINT16(40, power_cap_ma(&config, USB_LINK_UNCONFIGURED, 40));
}

static void test_suspend_beats_the_override(void)
{
    TEST_ASSERT_EQUAL_UINT16(0, power_cap_ma(&config, USB_LINK_SUSPENDED, 300));
}

static void test_estimate_is_duty_times_full_scale(void)
{
    const uint16_t red_full[LED_CHANNEL_COUNT] = { FULL, 0, 0 };
    const uint16_t white_full[LED_CHANNEL_COUNT] = { FULL, FULL, FULL };
    const uint16_t half_blue[LED_CHANNEL_COUNT] = { 0, 0, FULL / 2u };
    TEST_ASSERT_EQUAL_UINT32(860, power_duty_ma_x10(&config, red_full, FULL));
    TEST_ASSERT_EQUAL_UINT32(2740, power_duty_ma_x10(&config, white_full, FULL));
    TEST_ASSERT_EQUAL_UINT32(625, power_duty_ma_x10(&config, half_blue, FULL));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_cap_depends_on_the_usb_state);
    RUN_TEST(test_an_unknown_usb_state_allows_nothing);
    RUN_TEST(test_an_override_replaces_the_cap_of_the_link);
    RUN_TEST(test_suspend_beats_the_override);
    RUN_TEST(test_estimate_is_duty_times_full_scale);
    return UNITY_END();
}
