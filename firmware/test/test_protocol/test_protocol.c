/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <string.h>
#include <unity.h>

#include "protocol.h"

/* protocol/vectors/v2.json, turned into C by scripts/gen_vectors.py. */
#include "protocol_vectors.h"

static uint8_t payload[PROTO_REPORT_SIZE];
static proto_command_t command;

static void command_payload(uint8_t type, uint8_t seq, const uint8_t *body, uint8_t length)
{
    memset(payload, 0, sizeof payload);
    payload[0] = type;
    payload[1] = seq;
    payload[2] = length;
    if (length > 0u) {
        memcpy(&payload[3], body, length);
    }
}

void setUp(void)
{
    memset(&command, 0, sizeof command);
}

void tearDown(void)
{
}

static void test_decode_get_state(void)
{
    command_payload(PROTO_CMD_GET_STATE, 7, NULL, 0);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_UINT8(PROTO_CMD_GET_STATE, command.type);
    TEST_ASSERT_EQUAL_UINT8(7, command.seq);
}

static void test_decode_ignores_extra_body_bytes(void)
{
    /* Compatibility rule of version 1.1: a newer host may extend a body. */
    const uint8_t extra[] = { 0xAB };
    command_payload(PROTO_CMD_GET_INFO, 1, extra, sizeof extra);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));

    const uint8_t period[] = { 0xFA, 0x00, 0xAB };
    command_payload(PROTO_CMD_SET_TELEMETRY, 2, period, sizeof period);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_UINT16(250, command.body.telemetry_period_ms);

    /* A shorter body is still an error. */
    const uint8_t short_state[] = { LAMP_FIELD_POWER, 1, 0, 0 };
    command_payload(PROTO_CMD_SET_STATE, 3, short_state, sizeof short_state);
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_LENGTH, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_UINT8(3, command.seq);
}

static void test_decode_set_auto_bias_limits(void)
{
    const uint8_t plus_50[] = { 50 };
    command_payload(PROTO_CMD_SET_AUTO_BIAS, 1, plus_50, sizeof plus_50);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_INT8(50, command.body.auto_bias);

    const uint8_t minus_50[] = { 0xCE };
    command_payload(PROTO_CMD_SET_AUTO_BIAS, 1, minus_50, sizeof minus_50);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_INT8(-50, command.body.auto_bias);

    const uint8_t minus_128[] = { 0x80 };
    command_payload(PROTO_CMD_SET_AUTO_BIAS, 1, minus_128, sizeof minus_128);
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_VALUE, proto_decode_command(payload, &command));

    command_payload(PROTO_CMD_SET_AUTO_BIAS, 1, NULL, 0);
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_LENGTH, proto_decode_command(payload, &command));
}

/* The same limits and the same coding as the brightness bias. */
static void test_decode_set_touch_sensitivity_limits(void)
{
    const uint8_t plus_50[] = { 50 };
    command_payload(PROTO_CMD_SET_TOUCH_SENSITIVITY, 1, plus_50, sizeof plus_50);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_INT8(50, command.body.touch_sensitivity);

    const uint8_t minus_50[] = { 0xCE };
    command_payload(PROTO_CMD_SET_TOUCH_SENSITIVITY, 1, minus_50, sizeof minus_50);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_INT8(-50, command.body.touch_sensitivity);

    const uint8_t outside[] = { 51, 0xCD, 0x7F, 0x80 };
    for (size_t i = 0; i < sizeof outside; i++) {
        command_payload(PROTO_CMD_SET_TOUCH_SENSITIVITY, 1, &outside[i], 1);
        TEST_ASSERT_EQUAL(PROTO_ERR_BAD_VALUE, proto_decode_command(payload, &command));
    }

    command_payload(PROTO_CMD_SET_TOUCH_SENSITIVITY, 1, NULL, 0);
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_LENGTH, proto_decode_command(payload, &command));
}

static void test_decode_unknown_command(void)
{
    command_payload(0x42, 9, NULL, 0);
    TEST_ASSERT_EQUAL(PROTO_ERR_UNKNOWN_COMMAND, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_UINT8(0x42, command.type);
}

static void test_decode_rejects_an_oversized_length(void)
{
    command_payload(PROTO_CMD_GET_STATE, 1, NULL, 0);
    payload[2] = 61;
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_LENGTH, proto_decode_command(payload, &command));
}

/* SET_STATE bodies have 6 bytes, the touch mode last. */
static void test_decode_set_state(void)
{
    const uint8_t body[] = {
        LAMP_FIELD_ALL, 1, LAMP_COLOR_WARM_WHITE, BRIGHTNESS_MODE_MANUAL, 42, 1
    };
    command_payload(PROTO_CMD_SET_STATE, 3, body, sizeof body);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    const lamp_request_t *request = &command.body.set_state;
    TEST_ASSERT_EQUAL_UINT8(LAMP_FIELD_ALL, request->fields);
    TEST_ASSERT_TRUE(request->on);
    TEST_ASSERT_EQUAL(LAMP_COLOR_WARM_WHITE, request->color);
    TEST_ASSERT_EQUAL(BRIGHTNESS_MODE_MANUAL, request->brightness_mode);
    TEST_ASSERT_EQUAL_UINT8(42, request->manual_level);
    TEST_ASSERT_EQUAL(TOUCH_MODE_POWER_ONLY, request->touch_mode);
}

static void test_decode_set_state_bad_values(void)
{
    const uint8_t bad_color[] = { LAMP_FIELD_COLOR, 0, LAMP_SLOT_COUNT, 0, 0, 0 };
    command_payload(PROTO_CMD_SET_STATE, 1, bad_color, sizeof bad_color);
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_VALUE, proto_decode_command(payload, &command));

    const uint8_t bad_power[] = { LAMP_FIELD_POWER, 2, 0, 0, 0, 0 };
    command_payload(PROTO_CMD_SET_STATE, 1, bad_power, sizeof bad_power);
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_VALUE, proto_decode_command(payload, &command));

    const uint8_t bad_level[] = { LAMP_FIELD_MANUAL_LEVEL, 0, 0, 0, 0, 0 };
    command_payload(PROTO_CMD_SET_STATE, 1, bad_level, sizeof bad_level);
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_VALUE, proto_decode_command(payload, &command));
}

static void test_decode_set_state_ignores_unselected_fields(void)
{
    const uint8_t body[] = { LAMP_FIELD_COLOR, 0xFF, LAMP_COLOR_RED, 0xFF, 0xFF, 0xFF };
    command_payload(PROTO_CMD_SET_STATE, 1, body, sizeof body);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
}

static void test_decode_set_telemetry_limits(void)
{
    const uint8_t too_fast[] = { 10, 0 };
    command_payload(PROTO_CMD_SET_TELEMETRY, 1, too_fast, sizeof too_fast);
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_VALUE, proto_decode_command(payload, &command));

    const uint8_t stop[] = { 0, 0 };
    command_payload(PROTO_CMD_SET_TELEMETRY, 1, stop, sizeof stop);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_UINT16(0, command.body.telemetry_period_ms);

    const uint8_t every_250ms[] = { 0xFA, 0x00 };
    command_payload(PROTO_CMD_SET_TELEMETRY, 1, every_250ms, sizeof every_250ms);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_UINT16(250, command.body.telemetry_period_ms);
}

static void test_encode_state_bytes(void)
{
    const proto_state_t state = {
        .state_seq = 0x1234,
        .lamp = { .on = true,
                  .color = LAMP_COLOR_ORANGE,
                  .brightness_mode = BRIGHTNESS_MODE_MANUAL,
                  .manual_level = 30 },
        .applied_percent = 30,
        .link = USB_LINK_CONFIGURED,
        .source = CHANGE_SOURCE_TOUCH,
        .rgb = { 0x0102, 0x0304, 0x0506 },
    };
    proto_encode_state(payload, 5, &state);
    const uint8_t expected[] = {
        PROTO_MSG_STATE,
        5,
        PROTO_STATE_SIZE,
        0x34,
        0x12,
        1,
        1,
        1,
        30,
        30,
        1,
        1,
        0, /* touch mode */
        0x02,
        0x01,
        0x04,
        0x03,
        0x06,
        0x05,
    };
    TEST_ASSERT_EQUAL_HEX8_ARRAY(expected, payload, sizeof expected);
    for (size_t i = sizeof expected; i < PROTO_REPORT_SIZE; i++) {
        TEST_ASSERT_EQUAL_HEX8(0, payload[i]);
    }
}

static void test_encode_info_layout(void)
{
    const proto_info_t info = {
        .fw_major = 0,
        .fw_minor = 1,
        .fw_patch = 2,
        .board_revision = 4,
        .uid = { 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12 },
        .capabilities = PROTO_CAP_TELEMETRY | PROTO_CAP_DEBUG_CONSOLE,
        .reset_cause = RESET_CAUSE_POWER_ON,
        .flags = PROTO_INFO_FLAG_DEBUG_BUILD,
        .fault_mcause = 0x01020304,
        .fault_mepc = 0xA0B0C0D0,
        .fw_version = "0.1.2-3-gabcdef0",
    };
    proto_encode_info(payload, 9, &info);
    TEST_ASSERT_EQUAL_HEX8(PROTO_MSG_INFO, payload[0]);
    TEST_ASSERT_EQUAL_UINT8(9, payload[1]);
    TEST_ASSERT_EQUAL_UINT8(PROTO_INFO_SIZE, payload[2]);
    TEST_ASSERT_EQUAL_UINT8(PROTO_VERSION_MAJOR, payload[3]);
    TEST_ASSERT_EQUAL_UINT8(4, payload[8]);
    TEST_ASSERT_EQUAL_UINT8(1, payload[9]);
    TEST_ASSERT_EQUAL_UINT8(12, payload[20]);
    TEST_ASSERT_EQUAL_HEX8(0x03, payload[21]);
    TEST_ASSERT_EQUAL_UINT8(RESET_CAUSE_POWER_ON, payload[25]);
    TEST_ASSERT_EQUAL_HEX8(0x04, payload[27]);
    TEST_ASSERT_EQUAL_HEX8(0xD0, payload[31]);
    TEST_ASSERT_EQUAL_STRING("0.1.2-3-gabcdef0", (const char *)&payload[35]);
    TEST_ASSERT_EQUAL_HEX8(0, payload[3 + PROTO_INFO_SIZE]);
}

static void test_encode_info_truncates_a_long_version(void)
{
    const proto_info_t info = { .fw_version = "0123456789abcdefghijklmnopqrstuvwxyz" };
    proto_encode_info(payload, 0, &info);
    TEST_ASSERT_EQUAL_MEMORY("0123456789abcdefghijklmn", &payload[35], PROTO_FW_VERSION_SIZE);
    TEST_ASSERT_EQUAL_UINT8(PROTO_INFO_SIZE, payload[2]);
}

static void test_encode_telemetry_size_and_signed_delta(void)
{
    const proto_telemetry_t telemetry = { .touch_delta_permille = -2, .level_permille = 1000 };
    proto_encode_telemetry(payload, &telemetry);
    TEST_ASSERT_EQUAL_HEX8(PROTO_MSG_TELEMETRY, payload[0]);
    TEST_ASSERT_EQUAL_UINT8(0, payload[1]);
    TEST_ASSERT_EQUAL_UINT8(PROTO_TELEMETRY_SIZE, payload[2]);
    TEST_ASSERT_EQUAL_HEX8(0xFE, payload[3 + 10]);
    TEST_ASSERT_EQUAL_HEX8(0xFF, payload[3 + 11]);
    TEST_ASSERT_EQUAL_HEX8(0xE8, payload[3 + 30]);
    TEST_ASSERT_EQUAL_HEX8(0x03, payload[3 + 31]);
}

static void test_encode_ack_and_error(void)
{
    proto_encode_ack(payload, 4, PROTO_CMD_SET_TELEMETRY);
    const uint8_t ack[] = { PROTO_MSG_ACK, 4, 1, PROTO_CMD_SET_TELEMETRY };
    TEST_ASSERT_EQUAL_HEX8_ARRAY(ack, payload, sizeof ack);

    proto_encode_error(payload, 6, 0x42, PROTO_ERR_UNKNOWN_COMMAND);
    const uint8_t error[] = { PROTO_MSG_ERROR, 6, 2, 0x42, PROTO_ERR_UNKNOWN_COMMAND };
    TEST_ASSERT_EQUAL_HEX8_ARRAY(error, payload, sizeof error);
}

static void test_encode_diagnostics_layout(void)
{
    const proto_diagnostics_t diagnostics = {
        .uptime_ms = 1,
        .stack_free_min = 0x0102,
        .auto_bias = -1,
        .flags = PROTO_DIAG_FLAG_LIGHT_SENSOR | PROTO_DIAG_FLAG_CRYSTAL,
        .touch_sensitivity = -2,
    };
    proto_encode_diagnostics(payload, 7, &diagnostics);
    TEST_ASSERT_EQUAL_HEX8(PROTO_MSG_DIAGNOSTICS, payload[0]);
    TEST_ASSERT_EQUAL_UINT8(7, payload[1]);
    TEST_ASSERT_EQUAL_UINT8(PROTO_DIAGNOSTICS_SIZE, payload[2]);
    TEST_ASSERT_EQUAL_HEX8(0x02, payload[3 + 22]);
    TEST_ASSERT_EQUAL_HEX8(0x01, payload[3 + 23]);
    TEST_ASSERT_EQUAL_HEX8(0xFF, payload[3 + 34]);
    TEST_ASSERT_EQUAL_HEX8(0x03, payload[3 + 35]);
    TEST_ASSERT_EQUAL_UINT8(37, PROTO_DIAGNOSTICS_SIZE);
    TEST_ASSERT_EQUAL_HEX8(0xFE, payload[3 + 36]);
    TEST_ASSERT_EQUAL_HEX8(0, payload[3 + PROTO_DIAGNOSTICS_SIZE]);
}

static void test_encode_self_test_result_signed_peak(void)
{
    const proto_self_test_result_t result = { .results = PROTO_SELF_TEST_COMPLETED,
                                              .touch_peak_permille = -3 };
    proto_encode_self_test_result(payload, 8, &result);
    TEST_ASSERT_EQUAL_HEX8(PROTO_MSG_SELF_TEST_RESULT, payload[0]);
    TEST_ASSERT_EQUAL_UINT8(PROTO_SELF_TEST_RESULT_SIZE, payload[2]);
    TEST_ASSERT_EQUAL_HEX8(PROTO_SELF_TEST_COMPLETED, payload[3]);
    TEST_ASSERT_EQUAL_HEX8(0xFD, payload[3 + 7]);
    TEST_ASSERT_EQUAL_HEX8(0xFF, payload[3 + 8]);
}

static void test_vectors_match_the_protocol_version(void)
{
    TEST_ASSERT_EQUAL_UINT(PROTO_VERSION_MAJOR, VECTORS_PROTOCOL_MAJOR);
    TEST_ASSERT_EQUAL_UINT(PROTO_VERSION_MINOR, VECTORS_PROTOCOL_MINOR);
}

static void test_command_vectors(void)
{
    for (size_t i = 0; i < sizeof command_vectors / sizeof command_vectors[0]; i++) {
        const command_vector_t *v = &command_vectors[i];
        memset(&command, 0, sizeof command);
        TEST_ASSERT_EQUAL_MESSAGE(v->status, proto_decode_command(v->payload, &command), v->name);
        TEST_ASSERT_EQUAL_HEX8_MESSAGE(v->type, command.type, v->name);
        TEST_ASSERT_EQUAL_UINT8_MESSAGE(v->seq, command.seq, v->name);
        if (v->status != PROTO_OK) {
            continue;
        }
        if (v->has_set_state) {
            const lamp_request_t *got = &command.body.set_state;
            const lamp_request_t *want = &v->set_state;
            TEST_ASSERT_EQUAL_HEX8_MESSAGE(want->fields, got->fields, v->name);
            if (want->fields & LAMP_FIELD_POWER) {
                TEST_ASSERT_EQUAL_MESSAGE(want->on, got->on, v->name);
            }
            if (want->fields & LAMP_FIELD_COLOR) {
                TEST_ASSERT_EQUAL_MESSAGE(want->color, got->color, v->name);
            }
            if (want->fields & LAMP_FIELD_BRIGHTNESS_MODE) {
                TEST_ASSERT_EQUAL_MESSAGE(want->brightness_mode, got->brightness_mode, v->name);
            }
            if (want->fields & LAMP_FIELD_MANUAL_LEVEL) {
                TEST_ASSERT_EQUAL_UINT8_MESSAGE(want->manual_level, got->manual_level, v->name);
            }
        }
        if (v->has_period) {
            TEST_ASSERT_EQUAL_UINT16_MESSAGE(v->telemetry_period_ms,
                                             command.body.telemetry_period_ms, v->name);
        }
        if (v->has_auto_bias) {
            TEST_ASSERT_EQUAL_INT8_MESSAGE(v->auto_bias, command.body.auto_bias, v->name);
        }
        if (v->has_set_state && (v->set_state.fields & LAMP_FIELD_TOUCH_MODE) != 0u) {
            TEST_ASSERT_EQUAL_MESSAGE(v->set_state.touch_mode, command.body.set_state.touch_mode,
                                      v->name);
        }
        if (v->has_set_color) {
            TEST_ASSERT_EQUAL_UINT8_MESSAGE(v->set_color_slot, command.body.set_color.slot,
                                            v->name);
            TEST_ASSERT_EQUAL_MEMORY_MESSAGE(&v->set_color, &command.body.set_color.color,
                                             sizeof(rgb16_t), v->name);
        }
        if (v->has_color_slot) {
            TEST_ASSERT_EQUAL_UINT8_MESSAGE(v->color_slot, command.body.color_slot, v->name);
        }
        if (v->has_touch_cycle) {
            TEST_ASSERT_EQUAL_UINT8_MESSAGE(v->touch_cycle.count, command.body.touch_cycle.count,
                                            v->name);
            TEST_ASSERT_EQUAL_UINT8_ARRAY_MESSAGE(
                v->touch_cycle.slots, command.body.touch_cycle.slots, TOUCH_CYCLE_MAX, v->name);
        }
        if (v->has_alert) {
            TEST_ASSERT_EQUAL_UINT8_MESSAGE(v->alert_count, command.body.alert.count, v->name);
            if (v->alert_count > 0u) {
                TEST_ASSERT_EQUAL_UINT16_MESSAGE(v->alert_period_ms, command.body.alert.period_ms,
                                                 v->name);
            }
        }
        if (v->has_touch_sensitivity) {
            TEST_ASSERT_EQUAL_INT8_MESSAGE(v->touch_sensitivity, command.body.touch_sensitivity,
                                           v->name);
        }
    }
}

static void test_message_vectors(void)
{
    for (size_t i = 0; i < sizeof message_vectors / sizeof message_vectors[0]; i++) {
        const message_vector_t *v = &message_vectors[i];
        if (!v->canonical) {
            continue; /* decoder-only: the firmware never sends it */
        }
        switch (v->kind) {
            case MSG_STATE:
                proto_encode_state(payload, v->seq, &v->state);
                break;
            case MSG_INFO:
                proto_encode_info(payload, v->seq, &v->info);
                break;
            case MSG_TELEMETRY:
                proto_encode_telemetry(payload, &v->telemetry);
                break;
            case MSG_DIAGNOSTICS:
                proto_encode_diagnostics(payload, v->seq, &v->diagnostics);
                break;
            case MSG_SELF_TEST_RESULT:
                proto_encode_self_test_result(payload, v->seq, &v->self_test_result);
                break;
            case MSG_ACK:
                proto_encode_ack(payload, v->seq, v->command);
                break;
            case MSG_COLOR:
                proto_encode_color(payload, v->seq, v->color_slot, v->color);
                break;
            case MSG_TOUCH_CYCLE:
                proto_encode_touch_cycle(payload, v->seq, &v->touch_cycle);
                break;
            case MSG_ALERT_DONE:
                proto_encode_alert_done(payload, v->seq, v->reason);
                break;
            case MSG_ERROR:
            default:
                proto_encode_error(payload, v->seq, v->command, v->error);
                break;
        }
        TEST_ASSERT_EQUAL_HEX8_ARRAY_MESSAGE(v->payload, payload, PROTO_REPORT_SIZE, v->name);
    }
}

int main(void)
{
    UNITY_BEGIN();
    RUN_TEST(test_decode_get_state);
    RUN_TEST(test_decode_ignores_extra_body_bytes);
    RUN_TEST(test_decode_set_auto_bias_limits);
    RUN_TEST(test_decode_set_touch_sensitivity_limits);
    RUN_TEST(test_decode_unknown_command);
    RUN_TEST(test_decode_rejects_an_oversized_length);
    RUN_TEST(test_decode_set_state);
    RUN_TEST(test_decode_set_state_bad_values);
    RUN_TEST(test_decode_set_state_ignores_unselected_fields);
    RUN_TEST(test_decode_set_telemetry_limits);
    RUN_TEST(test_encode_state_bytes);
    RUN_TEST(test_encode_info_layout);
    RUN_TEST(test_encode_info_truncates_a_long_version);
    RUN_TEST(test_encode_telemetry_size_and_signed_delta);
    RUN_TEST(test_encode_ack_and_error);
    RUN_TEST(test_encode_diagnostics_layout);
    RUN_TEST(test_encode_self_test_result_signed_peak);
    RUN_TEST(test_vectors_match_the_protocol_version);
    RUN_TEST(test_command_vectors);
    RUN_TEST(test_message_vectors);
    return UNITY_END();
}
