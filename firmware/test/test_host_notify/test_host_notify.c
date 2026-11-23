/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * What the lamp sends the host on its own: STATE, with its sequence number,
 * its cause and the pace of the level changes, and the telemetry stream.
 */
#include <unity.h>

#include "host_notify.h"

#define LEVEL_PERIOD_MS 500u

static host_notify_t n;
static host_notify_state_t now;

void setUp(void)
{
    now = (host_notify_state_t){
        .applied_percent = 50,
        .link = USB_LINK_CONFIGURED,
        .xy = { 9748, 46140 },
    };
    lamp_fsm_init(&now.lamp);
    host_notify_init(&n, &now, LEVEL_PERIOD_MS);
}

void tearDown(void)
{
}

static void observe(change_source_t lamp_source)
{
    host_notify_observe(&n, &now, lamp_source);
}

static void test_the_boot_state_needs_no_message(void)
{
    observe(CHANGE_SOURCE_TOUCH);
    TEST_ASSERT_FALSE(host_notify_state_due(&n, 1000));
    const proto_state_t state = host_notify_state(&n);
    TEST_ASSERT_EQUAL_UINT16(0, state.state_seq);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_BOOT, state.source);
}

static void test_one_state_seq_step_per_state_with_new_content(void)
{
    now.lamp.color = LAMP_COLOR_ORANGE;
    observe(CHANGE_SOURCE_TOUCH);
    TEST_ASSERT_TRUE(host_notify_state_due(&n, 10));
    const proto_state_t state = host_notify_state(&n);
    TEST_ASSERT_EQUAL_UINT16(1, state.state_seq);
    TEST_ASSERT_EQUAL(LAMP_COLOR_ORANGE, state.lamp.color);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_TOUCH, state.source);
    host_notify_state_sent(&n, 10);
    TEST_ASSERT_FALSE(host_notify_state_due(&n, 20));

    /* An answer without a change repeats the number. */
    TEST_ASSERT_EQUAL_UINT16(1, host_notify_state(&n).state_seq);
    host_notify_state_sent(&n, 30);
    TEST_ASSERT_EQUAL_UINT16(1, host_notify_state(&n).state_seq);

    now.lamp.on = false;
    observe(CHANGE_SOURCE_HOST);
    TEST_ASSERT_EQUAL_UINT16(2, host_notify_state(&n).state_seq);
}

static void test_a_failed_send_takes_no_number(void)
{
    now.lamp.color = LAMP_COLOR_RED;
    observe(CHANGE_SOURCE_TOUCH);
    TEST_ASSERT_EQUAL_UINT16(1, host_notify_state(&n).state_seq);
    /* The send failed: still due, under the same number. */
    TEST_ASSERT_TRUE(host_notify_state_due(&n, 10));
    TEST_ASSERT_EQUAL_UINT16(1, host_notify_state(&n).state_seq);
    host_notify_state_sent(&n, 20);
    TEST_ASSERT_EQUAL_UINT16(1, host_notify_state(&n).state_seq);
    TEST_ASSERT_FALSE(host_notify_state_due(&n, 30));
}

/* While the answers fill the way, the changes wait and merge: one STATE, one
 * step, the latest content and the cause of the latest change. */
static void test_changes_merge_while_the_way_is_blocked(void)
{
    now.lamp.color = LAMP_COLOR_ORANGE;
    observe(CHANGE_SOURCE_TOUCH);
    now.lamp.color = LAMP_COLOR_RED;
    observe(CHANGE_SOURCE_TOUCH);
    now.applied_percent = 60;
    observe(CHANGE_SOURCE_TOUCH);

    const proto_state_t state = host_notify_state(&n);
    TEST_ASSERT_EQUAL_UINT16(1, state.state_seq);
    TEST_ASSERT_EQUAL(LAMP_COLOR_RED, state.lamp.color);
    TEST_ASSERT_EQUAL_UINT8(60, state.applied_percent);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_AUTO_BRIGHTNESS, state.source);
}

static void test_a_level_change_alone_waits_its_period(void)
{
    now.applied_percent = 51;
    observe(CHANGE_SOURCE_BOOT);
    TEST_ASSERT_TRUE(host_notify_state_due(&n, 1000));
    host_notify_state_sent(&n, 1000);

    now.applied_percent = 52;
    observe(CHANGE_SOURCE_BOOT);
    TEST_ASSERT_FALSE(host_notify_state_due(&n, 1100));

    /* Any other change leaves at once, and carries the level. */
    now.lamp.color = LAMP_COLOR_RED;
    observe(CHANGE_SOURCE_TOUCH);
    TEST_ASSERT_TRUE(host_notify_state_due(&n, 1100));
    TEST_ASSERT_EQUAL_UINT8(52, host_notify_state(&n).applied_percent);
    host_notify_state_sent(&n, 1100);

    /* The period counts from the last STATE of a level change alone. */
    now.applied_percent = 53;
    observe(CHANGE_SOURCE_BOOT);
    TEST_ASSERT_FALSE(host_notify_state_due(&n, 1499));
    TEST_ASSERT_TRUE(host_notify_state_due(&n, 1500));
}

static void test_the_cause_of_a_change(void)
{
    now.link = USB_LINK_SUSPENDED;
    observe(CHANGE_SOURCE_BOOT);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_USB, host_notify_state(&n).source);

    now.applied_percent = 10;
    observe(CHANGE_SOURCE_BOOT);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_AUTO_BRIGHTNESS, host_notify_state(&n).source);

    now.xy = (xy16_t){ 45170, 20347 }; /* the color shown, redefined */
    observe(CHANGE_SOURCE_BOOT);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_HOST, host_notify_state(&n).source);

    /* A change of the lamp state takes its own cause, whatever else changed. */
    now.link = USB_LINK_CONFIGURED;
    now.lamp.touch_mode = TOUCH_MODE_POWER_ONLY;
    observe(CHANGE_SOURCE_HOST_SILENCE);
    TEST_ASSERT_EQUAL(CHANGE_SOURCE_HOST_SILENCE, host_notify_state(&n).source);
}

static void test_the_telemetry_follows_its_period(void)
{
    TEST_ASSERT_FALSE(host_notify_take_telemetry(&n, 0));
    host_notify_set_telemetry(&n, 100, 1000);
    TEST_ASSERT_TRUE(host_notify_take_telemetry(&n, 1000));
    TEST_ASSERT_FALSE(host_notify_take_telemetry(&n, 1099));
    TEST_ASSERT_TRUE(host_notify_take_telemetry(&n, 1100));

    /* A late pass takes one sample, and the period counts from it. */
    TEST_ASSERT_TRUE(host_notify_take_telemetry(&n, 1350));
    TEST_ASSERT_FALSE(host_notify_take_telemetry(&n, 1449));
    TEST_ASSERT_TRUE(host_notify_take_telemetry(&n, 1450));

    host_notify_set_telemetry(&n, 0, 1500);
    TEST_ASSERT_FALSE(host_notify_take_telemetry(&n, 5000));
}

static void test_the_telemetry_stops_when_the_host_leaves_whatever_else_changed(void)
{
    const usb_link_t away[] = { USB_LINK_SUSPENDED, USB_LINK_UNCONFIGURED };
    for (size_t i = 0; i < sizeof away / sizeof away[0]; i++) {
        setUp();
        host_notify_set_telemetry(&n, 100, 0);
        /* The lamp changes at the same pass, as with a long press. */
        now.link = away[i];
        now.lamp.on = false;
        observe(CHANGE_SOURCE_TOUCH);
        TEST_ASSERT_FALSE(host_notify_take_telemetry(&n, 1000));

        /* Back on a configured link, the host asks again. */
        now.link = USB_LINK_CONFIGURED;
        observe(CHANGE_SOURCE_USB);
        TEST_ASSERT_FALSE(host_notify_take_telemetry(&n, 2000));
    }
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_the_boot_state_needs_no_message);
    RUN_TEST(test_one_state_seq_step_per_state_with_new_content);
    RUN_TEST(test_a_failed_send_takes_no_number);
    RUN_TEST(test_changes_merge_while_the_way_is_blocked);
    RUN_TEST(test_a_level_change_alone_waits_its_period);
    RUN_TEST(test_the_cause_of_a_change);
    RUN_TEST(test_the_telemetry_follows_its_period);
    RUN_TEST(test_the_telemetry_stops_when_the_host_leaves_whatever_else_changed);
    return UNITY_END();
}
