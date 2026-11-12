/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Codec of the USB HID protocol v2.0 (protocol/README.md), without I/O. A
 * payload is [0] type, [1] seq, [2] body length, then the body, zero padded,
 * with multi-byte fields in little-endian. INFO starts with the protocol
 * version, as in version 1, so that a host tells the two apart.
 */
#ifndef DISLED_PROTOCOL_H
#define DISLED_PROTOCOL_H

#include <stdbool.h>
#include <stddef.h>
#include <stdint.h>

#include "alert.h"
#include "disled_types.h"
#include "lamp_fsm.h"
#include "palette.h"
#include "touch_cycle.h"

#define PROTO_VERSION_MAJOR 2u
#define PROTO_VERSION_MINOR 0u

/* HID report IDs: commands go from the host to the device, messages from
 * the device to the host. */
#define PROTO_REPORT_ID_COMMAND 1u
#define PROTO_REPORT_ID_MESSAGE 2u

/* Payload bytes of one report, after the report ID. */
#define PROTO_REPORT_SIZE   63u
#define PROTO_HEADER_SIZE   3u
#define PROTO_MAX_BODY_SIZE (PROTO_REPORT_SIZE - PROTO_HEADER_SIZE)

/* From the host to the device. */
#define PROTO_CMD_GET_INFO      0x01u
#define PROTO_CMD_GET_STATE     0x02u
#define PROTO_CMD_SET_STATE     0x03u
#define PROTO_CMD_SET_TELEMETRY 0x04u
#define PROTO_CMD_RESET         0x05u
/* Version 1.1, each announced by a capability bit. */
#define PROTO_CMD_SET_AUTO_BIAS   0x06u
#define PROTO_CMD_GET_DIAGNOSTICS 0x07u
#define PROTO_CMD_SELF_TEST       0x08u
/* Version 2. 0x09 (ENTER_BOOTLOADER in version 1.1, which no lamp announced)
 * and 0x0F are reserved: unknown commands. */
#define PROTO_CMD_SET_COLOR             0x0Au
#define PROTO_CMD_GET_COLOR             0x0Bu
#define PROTO_CMD_SET_TOUCH_CYCLE       0x0Cu
#define PROTO_CMD_GET_TOUCH_CYCLE       0x0Du
#define PROTO_CMD_ALERT                 0x0Eu
#define PROTO_CMD_SET_TOUCH_SENSITIVITY 0x10u

/* Only in the builds for the HID bootloader prototype (firmware/bootloader/,
 * the _bl environments): ENTER_UPDATE takes 0x09 and capability bit 5. It
 * gets an ACK, then the lamp restarts into the bootloader, in update mode.
 * Every other build keeps both reserved, as protocol 2.0 says. */
#ifndef DISLED_ENTER_UPDATE
#define DISLED_ENTER_UPDATE 0
#endif
#if DISLED_ENTER_UPDATE
#define PROTO_CMD_ENTER_UPDATE 0x09u
#define PROTO_CAP_ENTER_UPDATE (1u << 5)
#endif

/* From the device to the host. */
#define PROTO_MSG_INFO             0x81u
#define PROTO_MSG_STATE            0x82u
#define PROTO_MSG_TELEMETRY        0x83u
#define PROTO_MSG_DIAGNOSTICS      0x84u
#define PROTO_MSG_SELF_TEST_RESULT 0x85u
/* Version 2. */
#define PROTO_MSG_COLOR       0x86u
#define PROTO_MSG_TOUCH_CYCLE 0x87u
#define PROTO_MSG_ALERT_DONE  0x88u
#define PROTO_MSG_ACK         0x8Eu
#define PROTO_MSG_ERROR       0x8Fu

/* Body sizes. Version 2 adds the touch mode to SET_STATE, and the touch mode
 * and the color of the lamp to STATE. */
#define PROTO_SET_STATE_SIZE 6u
#define PROTO_STATE_SIZE     16u
/* Version 2 sets a touch sensitivity correction, which ends DIAGNOSTICS. */
#define PROTO_SET_TOUCH_SENSITIVITY_SIZE 1u
#define PROTO_DIAGNOSTICS_SIZE           37u
#define PROTO_SET_TELEMETRY_SIZE         2u
#define PROTO_INFO_SIZE                  56u
#define PROTO_TELEMETRY_SIZE             32u
#define PROTO_SET_AUTO_BIAS_SIZE         1u
#define PROTO_SELF_TEST_RESULT_SIZE      9u
#define PROTO_SET_COLOR_SIZE             7u
#define PROTO_GET_COLOR_SIZE             1u
#define PROTO_COLOR_SIZE                 7u
#define PROTO_TOUCH_CYCLE_SIZE           10u
#define PROTO_ALERT_SIZE                 3u
#define PROTO_ALERT_DONE_SIZE            1u

/* INFO capabilities. */
#define PROTO_CAP_TELEMETRY     (1u << 0)
#define PROTO_CAP_DEBUG_CONSOLE (1u << 1)
#define PROTO_CAP_AUTO_BIAS     (1u << 2)
#define PROTO_CAP_DIAGNOSTICS   (1u << 3)
#define PROTO_CAP_SELF_TEST     (1u << 4)
/* Version 2. Bits 5 (ENTER_BOOTLOADER in version 1.1, never announced) and
 * 10 are reserved. */
#define PROTO_CAP_CUSTOM_COLORS     (1u << 6)
#define PROTO_CAP_TOUCH_CYCLE       (1u << 7)
#define PROTO_CAP_TOUCH_MODE        (1u << 8)
#define PROTO_CAP_ALERT             (1u << 9)
#define PROTO_CAP_TOUCH_SENSITIVITY (1u << 11)

/* ALERT_DONE reasons. */
#define PROTO_ALERT_COMPLETED 0u /* every breath shown */
#define PROTO_ALERT_LAMP_OFF  1u /* the lamp was turned off */
#define PROTO_ALERT_SUSPENDED 2u /* USB suspend */
#define PROTO_ALERT_CANCELLED 3u /* ALERT with a count of 0 */
#define PROTO_ALERT_SELF_TEST 4u /* SELF_TEST took the LED */

/* INFO flags. */
#define PROTO_INFO_FLAG_DEBUG_BUILD (1u << 0)
#define PROTO_INFO_FLAG_FAULT       (1u << 1)

/* DIAGNOSTICS flags. */
#define PROTO_DIAG_FLAG_LIGHT_SENSOR (1u << 0)
#define PROTO_DIAG_FLAG_CRYSTAL      (1u << 1)

/* SELF_TEST_RESULT results. */
#define PROTO_SELF_TEST_LIGHT_OK      (1u << 0)
#define PROTO_SELF_TEST_TOUCH_IDLE_OK (1u << 1)
#define PROTO_SELF_TEST_TOUCH_PRESSED (1u << 2)
#define PROTO_SELF_TEST_COMPLETED     (1u << 3)

/* Automatic brightness bias, percent. */
#define PROTO_AUTO_BIAS_MIN (-50)
#define PROTO_AUTO_BIAS_MAX 50

/* Touch sensitivity correction, percent (version 2). */
#define PROTO_TOUCH_SENSITIVITY_MIN (-50)
#define PROTO_TOUCH_SENSITIVITY_MAX 50

#define PROTO_FW_VERSION_SIZE 24u

/* Shortest telemetry period, in ms (0 stops the stream). */
#define PROTO_TELEMETRY_MIN_MS 20u

typedef enum {
    PROTO_OK = 0,
    PROTO_ERR_UNKNOWN_COMMAND = 1,
    PROTO_ERR_BAD_LENGTH = 2,
    PROTO_ERR_BAD_VALUE = 3,
    PROTO_ERR_BUSY = 4,     /* version 1.1 */
    PROTO_ERR_LAMP_OFF = 5, /* version 2: no alert while the lamp is off */
} proto_status_t;

typedef struct {
    uint8_t type;
    uint8_t seq;
    union {
        lamp_request_t set_state;
        uint16_t telemetry_period_ms;
        int8_t auto_bias; /* percent */
        struct {
            uint8_t slot; /* custom: PALETTE_FIXED_SLOTS and up */
            rgb16_t color;
        } set_color;
        uint8_t color_slot; /* GET_COLOR */
        /* Slots in range and none twice; the application checks they hold
         * a color. */
        touch_cycle_t touch_cycle;
        struct {
            uint8_t count; /* breaths; 0 stops a running alert */
            uint16_t period_ms;
        } alert;
        int8_t touch_sensitivity; /* percent */
    } body;
} proto_command_t;

typedef struct {
    uint8_t fw_major;
    uint8_t fw_minor;
    uint8_t fw_patch;
    uint8_t board_revision;
    uint8_t uid[12];
    uint32_t capabilities;
    reset_cause_t reset_cause;
    uint8_t flags;
    uint32_t fault_mcause;
    uint32_t fault_mepc;
    const char *fw_version; /* truncated to PROTO_FW_VERSION_SIZE */
} proto_info_t;

typedef struct {
    uint16_t state_seq;
    lamp_state_t lamp;
    uint8_t applied_percent;
    usb_link_t link;
    change_source_t source;
    rgb16_t rgb; /* the lamp color at full brightness, linear */
} proto_state_t;

typedef struct {
    uint32_t uptime_ms;
    uint16_t touch_raw;
    uint16_t touch_value;
    uint16_t touch_baseline;
    int16_t touch_delta_permille;
    uint8_t touch_state;
    uint32_t lux_centi;
    uint16_t als_raw;
    uint16_t duty[LED_CHANNEL_COUNT];
    uint16_t led_current_ma_x10;
    uint16_t cap_ma;
    usb_link_t link;
    uint16_t level_permille;
} proto_telemetry_t;

typedef struct {
    uint32_t uptime_ms;
    uint16_t boots;           /* since power-up */
    uint16_t watchdog_resets; /* since power-up */
    uint16_t faults;          /* since power-up */
    uint32_t fault_mcause;    /* last fault, 0 without fault */
    uint32_t fault_mepc;
    uint32_t fault_mtval;
    uint16_t stack_free_min; /* bytes, since boot */
    uint16_t i2c_errors;
    uint16_t light_sensor_probes;
    uint16_t touch_samples_lost;
    uint16_t touch_recalibrations;
    uint16_t usb_suspends;
    int8_t auto_bias;         /* percent */
    uint8_t flags;            /* PROTO_DIAG_FLAG_* */
    int8_t touch_sensitivity; /* percent */
} proto_diagnostics_t;

typedef struct {
    uint8_t results; /* PROTO_SELF_TEST_* */
    uint32_t lux_centi;
    uint16_t touch_baseline;
    int16_t touch_peak_permille;
} proto_self_test_result_t;

/* Extra body bytes are ignored, so that newer hosts can extend a command. On
 * error, `command->type` and `command->seq` are still set, for the reply. */
proto_status_t proto_decode_command(const uint8_t payload[PROTO_REPORT_SIZE],
                                    proto_command_t *command);

void proto_encode_info(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, const proto_info_t *info);
void proto_encode_state(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq,
                        const proto_state_t *state);
void proto_encode_telemetry(uint8_t payload[PROTO_REPORT_SIZE], const proto_telemetry_t *telemetry);
void proto_encode_diagnostics(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq,
                              const proto_diagnostics_t *diagnostics);
void proto_encode_self_test_result(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq,
                                   const proto_self_test_result_t *result);
void proto_encode_ack(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, uint8_t command_type);
void proto_encode_error(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, uint8_t command_type,
                        proto_status_t error);
void proto_encode_color(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, uint8_t slot,
                        rgb16_t color);
void proto_encode_touch_cycle(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq,
                              const touch_cycle_t *cycle);
/* `seq` is that of the ALERT command; `reason` one of PROTO_ALERT_*. */
void proto_encode_alert_done(uint8_t payload[PROTO_REPORT_SIZE], uint8_t seq, uint8_t reason);

#endif /* DISLED_PROTOCOL_H */
