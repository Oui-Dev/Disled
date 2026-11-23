/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <string.h>
#include <unity.h>

#include "protocol.h"
#include "self_test.h"

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

static void test_decode_ignores_extra_body_bytes(void)
{
    /* A newer host may extend a body. */
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

static void test_decode_rejects_an_oversized_length(void)
{
    command_payload(PROTO_CMD_GET_STATE, 1, NULL, 0);
    payload[2] = 61;
    TEST_ASSERT_EQUAL(PROTO_ERR_BAD_LENGTH, proto_decode_command(payload, &command));
}

/* 0 stops the stream, though it lies below the shortest period. */
static void test_decode_set_telemetry_stop(void)
{
    const uint8_t stop[] = { 0, 0 };
    command_payload(PROTO_CMD_SET_TELEMETRY, 1, stop, sizeof stop);
    TEST_ASSERT_EQUAL(PROTO_OK, proto_decode_command(payload, &command));
    TEST_ASSERT_EQUAL_UINT16(0, command.body.telemetry_period_ms);
}

static void test_encode_info_truncates_a_long_version(void)
{
    const proto_info_t info = { .fw_version = "0123456789abcdefghijklmnopqrstuvwxyz" };
    proto_encode_info(payload, 0, &info);
    TEST_ASSERT_EQUAL_MEMORY("0123456789abcdefghijklmn", &payload[35], PROTO_FW_VERSION_SIZE);
    TEST_ASSERT_EQUAL_UINT8(PROTO_INFO_SIZE, payload[2]);
}

static void test_encode_self_test_result_signed_peak(void)
{
    const proto_self_test_result_t result = { .results = SELF_TEST_COMPLETED,
                                              .touch_peak_permille = -3 };
    proto_encode_self_test_result(payload, 8, &result);
    TEST_ASSERT_EQUAL_HEX8(PROTO_MSG_SELF_TEST_RESULT, payload[0]);
    TEST_ASSERT_EQUAL_UINT8(PROTO_SELF_TEST_RESULT_SIZE, payload[2]);
    TEST_ASSERT_EQUAL_HEX8(SELF_TEST_COMPLETED, payload[3]);
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
            if (want->fields & LAMP_FIELD_TOUCH_MODE) {
                TEST_ASSERT_EQUAL_MESSAGE(want->touch_mode, got->touch_mode, v->name);
            }
        }
        if (v->has_period) {
            TEST_ASSERT_EQUAL_UINT16_MESSAGE(v->telemetry_period_ms,
                                             command.body.telemetry_period_ms, v->name);
        }
        if (v->has_auto_bias) {
            TEST_ASSERT_EQUAL_INT8_MESSAGE(v->auto_bias, command.body.auto_bias, v->name);
        }
        if (v->has_set_color) {
            TEST_ASSERT_EQUAL_UINT8_MESSAGE(v->set_color_slot, command.body.set_color.slot,
                                            v->name);
            TEST_ASSERT_EQUAL_MEMORY_MESSAGE(&v->set_color, &command.body.set_color.color,
                                             sizeof(xy16_t), v->name);
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
            case MSG_TOUCH:
                proto_encode_touch(payload, v->press);
                break;
            case MSG_GAMUT:
                proto_encode_gamut(payload, v->seq, v->gamut);
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
    RUN_TEST(test_decode_ignores_extra_body_bytes);
    RUN_TEST(test_decode_set_auto_bias_limits);
    RUN_TEST(test_decode_rejects_an_oversized_length);
    RUN_TEST(test_decode_set_telemetry_stop);
    RUN_TEST(test_encode_info_truncates_a_long_version);
    RUN_TEST(test_encode_self_test_result_signed_peak);
    RUN_TEST(test_vectors_match_the_protocol_version);
    RUN_TEST(test_command_vectors);
    RUN_TEST(test_message_vectors);
    return UNITY_END();
}
