/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Lamp state machine: power, color and brightness settings, from the touch
 * gestures and the host requests (docs/behavior.md). USB suspend stays out of
 * it: the application drops the gestures and the LED output goes dark.
 */
#ifndef DISLED_LAMP_FSM_H
#define DISLED_LAMP_FSM_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"
#include "touch_cycle.h"

/* Manual level used until the host chooses one. */
#define LAMP_DEFAULT_MANUAL_LEVEL 50u

typedef struct {
    bool on;
    /* Shown when on; kept while off so that the lamp comes back with it. A
     * palette slot with custom colors. */
    lamp_color_t color;
    brightness_mode_t brightness_mode;
    /* Percent, BRIGHTNESS_MANUAL_MIN to BRIGHTNESS_MANUAL_MAX. */
    uint8_t manual_level;
    touch_mode_t touch_mode;
} lamp_state_t;

/* Fields of a host request. The values are part of the protocol. */
#define LAMP_FIELD_POWER           0x01u
#define LAMP_FIELD_COLOR           0x02u
#define LAMP_FIELD_BRIGHTNESS_MODE 0x04u
#define LAMP_FIELD_MANUAL_LEVEL    0x08u
#define LAMP_FIELD_TOUCH_MODE      0x10u
#define LAMP_FIELD_ALL             0x1Fu

typedef struct {
    uint8_t fields; /* LAMP_FIELD_* bits: which members below apply */
    bool on;
    lamp_color_t color;
    brightness_mode_t brightness_mode;
    uint8_t manual_level;
    touch_mode_t touch_mode;
} lamp_request_t;

/* Power-up state: on, green, automatic brightness, normal touch. */
void lamp_fsm_init(lamp_state_t *state);

/* The next color while on (docs/behavior.md), nothing while off or in
 * power-only touch mode. Returns true if the state changed. */
bool lamp_fsm_short_press(lamp_state_t *state, const touch_cycle_t *cycle);

/* Power-only mode lasts while a host drives the lamp: back to normal once the
 * link leaves the configured state, or after `timeout_ms` without a command
 * (`silent_ms` so far), so that a crashed app cannot leave the lid dead.
 * Returns true if the mode changed, with the cause in `source`. */
bool lamp_fsm_touch_fallback(lamp_state_t *state, usb_link_t link, uint32_t silent_ms,
                             uint32_t timeout_ms, change_source_t *source);

/* Toggles the power, keeping the color: the state always changes. */
void lamp_fsm_long_press(lamp_state_t *state);

/* True if every selected field holds a valid value and no unknown field
 * bit is set. With custom colors, the application also checks that the
 * color slot holds a color. */
bool lamp_request_is_valid(const lamp_request_t *request);

bool lamp_state_equal(const lamp_state_t *a, const lamp_state_t *b);

/* Applies a valid request atomically and returns true if the state changed.
 * An invalid request changes nothing and returns false. */
bool lamp_fsm_apply(lamp_state_t *state, const lamp_request_t *request);

#endif /* DISLED_LAMP_FSM_H */
