/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "protocol.h"

#include <string.h>

_Static_assert(PROTO_INFO_SIZE <= PROTO_MAX_BODY_SIZE, "INFO fits one report");
_Static_assert(PROTO_TELEMETRY_SIZE <= PROTO_MAX_BODY_SIZE, "TELEMETRY fits one report");
_Static_assert(PROTO_DIAGNOSTICS_SIZE <= PROTO_MAX_BODY_SIZE, "DIAGNOSTICS fits one report");
_Static_assert(LAMP_FIELD_POWER == 0x01u && LAMP_FIELD_COLOR == 0x02u &&
                   LAMP_FIELD_BRIGHTNESS_MODE == 0x04u && LAMP_FIELD_MANUAL_LEVEL == 0x08u,
               "SET_STATE field bits are part of the protocol");
_Static_assert(LAMP_FIELD_TOUCH_MODE == 0x10u, "SET_STATE field bits are part of the protocol");
_Static_assert(1u + TOUCH_CYCLE_MAX == PROTO_TOUCH_CYCLE_SIZE, "TOUCH_CYCLE: count and slots");

/* Little-endian writer over one payload. */
typedef struct {
    uint8_t *payload;
    size_t pos;
} writer_t;

static writer_t begin_message(uint8_t payload[PROTO_REPORT_SIZE], uint8_t type, uint8_t seq)
{
    memset(payload, 0, PROTO_REPORT_SIZE);
    payload[0] = type;
    payload[1] = seq;
    const writer_t w = { payload, PROTO_HEADER_SIZE };
    return w;
}

static void end_message(writer_t *w)
{
    w->payload[2] = (uint8_t)(w->pos - PROTO_HEADER_SIZE);
}

static void put_u8(writer_t *w, uint8_t value)
{
    w->payload[w->pos++] = value;
}

static void put_u16(writer_t *w, uint16_t value)
{
    put_u8(w, (uint8_t)(value & 0xFFu));
    put_u8(w, (uint8_t)(value >> 8));
}

static void put_u32(writer_t *w, uint32_t value)
{
    put_u16(w, (uint16_t)(value & 0xFFFFu));
    put_u16(w, (uint16_t)(value >> 16));
}

static uint16_t get_u16(const uint8_t *bytes)
{
    return (uint16_t)(bytes[0] | ((uint16_t)bytes[1] << 8));
}

/* Two's complement to a signed value without an implementation-defined conversion. */
static int32_t get_i8(const uint8_t *bytes)
{
    return (bytes[0] < 0x80u) ? (int32_t)bytes[0] : (int32_t)bytes[0] - 256;
}

proto_status_t proto_decode_command(const uint8_t payload[PROTO_REPORT_SIZE],
                                    proto_command_t *command)
{
    command->type = payload[0];
    command->seq = payload[1];
    const uint8_t length = payload[2];
    const uint8_t *body = &payload[PROTO_HEADER_SIZE];

    if (length > PROTO_MAX_BODY_SIZE) {
        return PROTO_ERR_BAD_LENGTH;
    }

    switch (command->type) {
        case PROTO_CMD_GET_INFO:
        case PROTO_CMD_GET_STATE:
        case PROTO_CMD_RESET:
        case PROTO_CMD_GET_DIAGNOSTICS:
        case PROTO_CMD_SELF_TEST:
            return PROTO_OK;

        case PROTO_CMD_SET_STATE: {
            if (length < PROTO_SET_STATE_SIZE) {
                return PROTO_ERR_BAD_LENGTH;
            }
            /* Fields that are not selected are ignored, whatever their value. */
            if ((body[0] & LAMP_FIELD_POWER) != 0u && body[1] > 1u) {
                return PROTO_ERR_BAD_VALUE;
            }
            lamp_request_t *request = &command->body.set_state;
            request->fields = body[0];
            request->on = body[1] != 0u;
            request->color = (lamp_color_t)body[2];
            request->brightness_mode = (brightness_mode_t)body[3];
            request->manual_level = body[4];
            request->touch_mode = (touch_mode_t)body[5];
            return lamp_request_is_valid(request) ? PROTO_OK : PROTO_ERR_BAD_VALUE;
        }

        case PROTO_CMD_SET_TELEMETRY: {
            if (length < PROTO_SET_TELEMETRY_SIZE) {
                return PROTO_ERR_BAD_LENGTH;
            }
            const uint16_t period = get_u16(body);
            if (period != 0u && period < PROTO_TELEMETRY_MIN_MS) {
                return PROTO_ERR_BAD_VALUE;
            }
            command->body.telemetry_period_ms = period;
            return PROTO_OK;
        }

        case PROTO_CMD_SET_AUTO_BIAS: {
            if (length < PROTO_SET_AUTO_BIAS_SIZE) {
                return PROTO_ERR_BAD_LENGTH;
            }
            const int32_t bias = get_i8(body);
            if (bias < PROTO_AUTO_BIAS_MIN || bias > PROTO_AUTO_BIAS_MAX) {
                return PROTO_ERR_BAD_VALUE;
            }
            command->body.auto_bias = (int8_t)bias;
            return PROTO_OK;
        }

        case PROTO_CMD_SET_COLOR: {
            if (length < PROTO_SET_COLOR_SIZE) {
                return PROTO_ERR_BAD_LENGTH;
            }
            const rgb16_t color = { get_u16(&body[1]), get_u16(&body[3]), get_u16(&body[5]) };
            /* The built-in colors never change. */
            if (body[0] < PALETTE_FIXED_SLOTS || body[0] >= PALETTE_SLOTS ||
                !palette_color_is_valid(color)) {
                return PROTO_ERR_BAD_VALUE;
            }
            command->body.set_color.slot = body[0];
            command->body.set_color.color = color;
            return PROTO_OK;
        }

        case PROTO_CMD_GET_COLOR:
            if (length < PROTO_GET_COLOR_SIZE) {
                return PROTO_ERR_BAD_LENGTH;
            }
            if (body[0] >= PALETTE_SLOTS) {
                return PROTO_ERR_BAD_VALUE;
            }
            command->body.color_slot = body[0];
            return PROTO_OK;

        case PROTO_CMD_SET_TOUCH_CYCLE: {
            if (length < 1u) {
                return PROTO_ERR_BAD_LENGTH;
            }
            const uint8_t count = body[0];
            if (count == 0u || count > TOUCH_CYCLE_MAX) {
                return PROTO_ERR_BAD_VALUE;
            }
            if (length < 1u + count) {
                return PROTO_ERR_BAD_LENGTH;
            }
            /* Every slot passes here; the application knows which hold a color. */
            touch_cycle_t *cycle = &command->body.touch_cycle;
            const uint32_t every_slot = (1u << LAMP_SLOT_COUNT) - 1u;
            return touch_cycle_set(cycle, &body[1], count, every_slot) ? PROTO_OK
                                                                       : PROTO_ERR_BAD_VALUE;
        }

        case PROTO_CMD_GET_TOUCH_CYCLE:
            return PROTO_OK;

        case PROTO_CMD_ALERT: {
            if (length < PROTO_ALERT_SIZE) {
                return PROTO_ERR_BAD_LENGTH;
            }
            const uint8_t count = body[0];
            const uint16_t period = get_u16(&body[1]);
            /* A count of 0 stops the alert, whatever the period. */
            if (count > ALERT_COUNT_MAX ||
                (count > 0u && (period < ALERT_PERIOD_MIN_MS || period > ALERT_PERIOD_MAX_MS))) {
                return PROTO_ERR_BAD_VALUE;
            }
            command->body.alert.count = count;
            command->body.alert.period_ms = period;
            return PROTO_OK;
        }

        case PROTO_CMD_SET_TOUCH_SENSITIVITY: {
            if (length < PROTO_SET_TOUCH_SENSITIVITY_SIZE) {
                return PROTO_ERR_BAD_LENGTH;
            }
            const int32_t percent = get_i8(body);
            if (percent < PROTO_TOUCH_SENSITIVITY_MIN || percent > PROTO_TOUCH_SENSITIVITY_MAX) {
                return PROTO_ERR_BAD_VALUE;
            }
            command->body.touch_sensitivity = (int8_t)percent;
            return PROTO_OK;
        }

        default:
            return PROTO_ERR_UNKNOWN_COMMAND;
    }
}

void proto_encode_info(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, const proto_info_t *info)
{
    writer_t w = begin_message(payload, PROTO_MSG_INFO, seq);
    put_u8(&w, PROTO_VERSION_MAJOR);
    put_u8(&w, PROTO_VERSION_MINOR);
    put_u8(&w, info->fw_major);
    put_u8(&w, info->fw_minor);
    put_u8(&w, info->fw_patch);
    put_u8(&w, info->board_revision);
    for (size_t i = 0; i < sizeof info->uid; i++) {
        put_u8(&w, info->uid[i]);
    }
    put_u32(&w, info->capabilities);
    put_u8(&w, (uint8_t)info->reset_cause);
    put_u8(&w, info->flags);
    put_u32(&w, info->fault_mcause);
    put_u32(&w, info->fault_mepc);
    const char *version = (info->fw_version != NULL) ? info->fw_version : "";
    for (size_t i = 0; i < PROTO_FW_VERSION_SIZE; i++) {
        const uint8_t c = (uint8_t)version[i];
        put_u8(&w, c);
        if (c == 0u) {
            /* The rest of the field is already zero. */
            w.pos += PROTO_FW_VERSION_SIZE - i - 1u;
            break;
        }
    }
    end_message(&w);
}

void proto_encode_state(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, const proto_state_t *state)
{
    writer_t w = begin_message(payload, PROTO_MSG_STATE, seq);
    put_u16(&w, state->state_seq);
    put_u8(&w, state->lamp.on ? 1u : 0u);
    put_u8(&w, (uint8_t)state->lamp.color);
    put_u8(&w, (uint8_t)state->lamp.brightness_mode);
    put_u8(&w, state->lamp.manual_level);
    put_u8(&w, state->applied_percent);
    put_u8(&w, (uint8_t)state->link);
    put_u8(&w, (uint8_t)state->source);
    put_u8(&w, (uint8_t)state->lamp.touch_mode);
    put_u16(&w, state->rgb.r);
    put_u16(&w, state->rgb.g);
    put_u16(&w, state->rgb.b);
    end_message(&w);
}

void proto_encode_telemetry(uint8_t payload[PROTO_REPORT_SIZE], const proto_telemetry_t *telemetry)
{
    writer_t w = begin_message(payload, PROTO_MSG_TELEMETRY, 0u);
    put_u32(&w, telemetry->uptime_ms);
    put_u16(&w, telemetry->touch_raw);
    put_u16(&w, telemetry->touch_value);
    put_u16(&w, telemetry->touch_baseline);
    put_u16(&w, (uint16_t)telemetry->touch_delta_permille);
    put_u8(&w, telemetry->touch_state);
    put_u32(&w, telemetry->lux_centi);
    put_u16(&w, telemetry->als_raw);
    for (size_t i = 0; i < LED_CHANNEL_COUNT; i++) {
        put_u16(&w, telemetry->duty[i]);
    }
    put_u16(&w, telemetry->led_current_ma_x10);
    put_u16(&w, telemetry->cap_ma);
    put_u8(&w, (uint8_t)telemetry->link);
    put_u16(&w, telemetry->level_permille);
    end_message(&w);
}

void proto_encode_diagnostics(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq,
                              const proto_diagnostics_t *diagnostics)
{
    writer_t w = begin_message(payload, PROTO_MSG_DIAGNOSTICS, seq);
    put_u32(&w, diagnostics->uptime_ms);
    put_u16(&w, diagnostics->boots);
    put_u16(&w, diagnostics->watchdog_resets);
    put_u16(&w, diagnostics->faults);
    put_u32(&w, diagnostics->fault_mcause);
    put_u32(&w, diagnostics->fault_mepc);
    put_u32(&w, diagnostics->fault_mtval);
    put_u16(&w, diagnostics->stack_free_min);
    put_u16(&w, diagnostics->i2c_errors);
    put_u16(&w, diagnostics->light_sensor_probes);
    put_u16(&w, diagnostics->touch_samples_lost);
    put_u16(&w, diagnostics->touch_recalibrations);
    put_u16(&w, diagnostics->usb_suspends);
    put_u8(&w, (uint8_t)diagnostics->auto_bias);
    put_u8(&w, diagnostics->flags);
    put_u8(&w, (uint8_t)diagnostics->touch_sensitivity);
    end_message(&w);
}

void proto_encode_self_test_result(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq,
                                   const proto_self_test_result_t *result)
{
    writer_t w = begin_message(payload, PROTO_MSG_SELF_TEST_RESULT, seq);
    put_u8(&w, result->results);
    put_u32(&w, result->lux_centi);
    put_u16(&w, result->touch_baseline);
    put_u16(&w, (uint16_t)result->touch_peak_permille);
    end_message(&w);
}

void proto_encode_ack(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, uint8_t command_type)
{
    writer_t w = begin_message(payload, PROTO_MSG_ACK, seq);
    put_u8(&w, command_type);
    end_message(&w);
}

void proto_encode_error(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, uint8_t command_type,
                        proto_status_t error)
{
    writer_t w = begin_message(payload, PROTO_MSG_ERROR, seq);
    put_u8(&w, command_type);
    put_u8(&w, (uint8_t)error);
    end_message(&w);
}

void proto_encode_color(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, uint8_t slot,
                        rgb16_t color)
{
    writer_t w = begin_message(payload, PROTO_MSG_COLOR, seq);
    put_u8(&w, slot);
    put_u16(&w, color.r);
    put_u16(&w, color.g);
    put_u16(&w, color.b);
    end_message(&w);
}

void proto_encode_touch_cycle(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq,
                              const touch_cycle_t *cycle)
{
    writer_t w = begin_message(payload, PROTO_MSG_TOUCH_CYCLE, seq);
    put_u8(&w, cycle->count);
    for (size_t i = 0; i < TOUCH_CYCLE_MAX; i++) {
        put_u8(&w, cycle->slots[i]); /* zero past the count */
    }
    end_message(&w);
}

void proto_encode_alert_done(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, uint8_t reason)
{
    writer_t w = begin_message(payload, PROTO_MSG_ALERT_DONE, seq);
    put_u8(&w, reason);
    end_message(&w);
}
