/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * LED current budget by USB state; led_output applies it (test_led_output).
 */
#include <unity.h>

#include "power_limiter.h"

/* Worst-case full-scale currents (constraint N2 in hardware/README.md). */
static const power_config_t config = {
    .full_scale_ma = { 86, 63, 125 },
    .unconfigured_cap_ma = 70,
    .configured_cap_ma = 125,
};

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

static void test_an_unknown_usb_state_allows_nothing(void)
{
    TEST_ASSERT_EQUAL_UINT16(0, power_cap_ma(&config, (usb_link_t)3));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_cap_depends_on_the_usb_state);
    RUN_TEST(test_an_unknown_usb_state_allows_nothing);
    return UNITY_END();
}
