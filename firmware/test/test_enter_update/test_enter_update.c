/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * The decoder of the builds for the HID bootloader prototype, run by env
 * native_bl (DISLED_ENTER_UPDATE=1). The other builds keep 0x09 reserved,
 * which the vectors of test_protocol check.
 */
#include <string.h>
#include <unity.h>

#include "protocol.h"

#if !DISLED_ENTER_UPDATE
#error "test_enter_update runs in env native_bl only"
#endif

static uint8_t payload[PROTO_REPORT_SIZE];
static proto_command_t command;

void setUp(void)
{
    memset(payload, 0, sizeof payload);
    memset(&command, 0, sizeof command);
}

void tearDown(void)
{
}

static void test_enter_update_takes_0x09(void)
{
    payload[0] = PROTO_CMD_ENTER_UPDATE;
    payload[1] = 9;
    TEST_ASSERT_EQUAL_HEX8(0x09u, PROTO_CMD_ENTER_UPDATE);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_UINT8(PROTO_CMD_ENTER_UPDATE, command.type);
    TEST_ASSERT_EQUAL_UINT8(9, command.seq);
}

static void test_0x0f_stays_reserved(void)
{
    payload[0] = 0x0Fu;
    payload[1] = 1;
    TEST_ASSERT_EQUAL(PROTO_ERR_UNKNOWN_COMMAND, proto_decode_command(payload, &command));
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_enter_update_takes_0x09);
    RUN_TEST(test_0x0f_stays_reserved);
    return UNITY_END();
}
