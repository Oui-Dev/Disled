/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "lamp_fsm.h"

void lamp_fsm_init(lamp_state_t *state)
{
    state->on = true;
    state->color = LAMP_COLOR_GREEN;
    state->brightness_mode = BRIGHTNESS_MODE_AUTO;
    state->manual_level = LAMP_DEFAULT_MANUAL_LEVEL;
    state->touch_mode = TOUCH_MODE_NORMAL;
}

bool lamp_fsm_short_press(lamp_state_t *state, const touch_cycle_t *cycle)
{
    if (state->touch_mode == TOUCH_MODE_POWER_ONLY) {
        return false;
    }
    if (!state->on) {
        return false;
    }
    const lamp_color_t next = touch_cycle_next(cycle, state->color);
    const bool changed = next != state->color;
    state->color = next;
    return changed;
}

void lamp_fsm_long_press(lamp_state_t *state)
{
    state->on = !state->on;
}

bool lamp_fsm_touch_fallback(lamp_state_t *state, usb_link_t link, uint32_t silent_ms,
                             uint32_t timeout_ms, change_source_t *source)
{
    if (state->touch_mode == TOUCH_MODE_NORMAL) {
        return false;
    }
    if (link != USB_LINK_CONFIGURED) {
        *source = CHANGE_SOURCE_USB;
    } else if (silent_ms >= timeout_ms) {
        *source = CHANGE_SOURCE_HOST_SILENCE;
    } else {
        return false;
    }
    state->touch_mode = TOUCH_MODE_NORMAL;
    return true;
}

bool lamp_request_is_valid(const lamp_request_t *request)
{
    if ((request->fields & (uint8_t)~LAMP_FIELD_ALL) != 0u) {
        return false;
    }
    if ((request->fields & LAMP_FIELD_COLOR) != 0u && (uint32_t)request->color >= LAMP_SLOT_COUNT) {
        return false;
    }
    if ((request->fields & LAMP_FIELD_TOUCH_MODE) != 0u &&
        (uint32_t)request->touch_mode >= TOUCH_MODE_COUNT) {
        return false;
    }
    if ((request->fields & LAMP_FIELD_BRIGHTNESS_MODE) != 0u &&
        request->brightness_mode != BRIGHTNESS_MODE_AUTO &&
        request->brightness_mode != BRIGHTNESS_MODE_MANUAL) {
        return false;
    }
    if ((request->fields & LAMP_FIELD_MANUAL_LEVEL) != 0u &&
        (request->manual_level < BRIGHTNESS_MANUAL_MIN ||
         request->manual_level > BRIGHTNESS_MANUAL_MAX)) {
        return false;
    }
    return true;
}

bool lamp_state_equal(const lamp_state_t *a, const lamp_state_t *b)
{
    if (a->touch_mode != b->touch_mode) {
        return false;
    }
    return a->on == b->on && a->color == b->color && a->brightness_mode == b->brightness_mode &&
           a->manual_level == b->manual_level;
}

bool lamp_fsm_apply(lamp_state_t *state, const lamp_request_t *request)
{
    if (!lamp_request_is_valid(request)) {
        return false;
    }

    lamp_state_t next = *state;
    if ((request->fields & LAMP_FIELD_POWER) != 0u) {
        next.on = request->on;
    }
    if ((request->fields & LAMP_FIELD_COLOR) != 0u) {
        next.color = request->color;
    }
    if ((request->fields & LAMP_FIELD_BRIGHTNESS_MODE) != 0u) {
        next.brightness_mode = request->brightness_mode;
    }
    if ((request->fields & LAMP_FIELD_MANUAL_LEVEL) != 0u) {
        next.manual_level = request->manual_level;
    }
    if ((request->fields & LAMP_FIELD_TOUCH_MODE) != 0u) {
        next.touch_mode = request->touch_mode;
    }

    const bool changed = !lamp_state_equal(&next, state);
    *state = next;
    return changed;
}
