/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "host_notify.h"

#include "time_utils.h"

static bool same_xy(xy16_t a, xy16_t b)
{
    return a.x == b.x && a.y == b.y;
}

/* Everything but the applied level. */
static bool same_but_level(const host_notify_state_t *a, const host_notify_state_t *b)
{
    return same_xy(a->xy, b->xy) && lamp_state_equal(&a->lamp, &b->lamp) && a->link == b->link;
}

static bool same_content(const host_notify_state_t *a, const host_notify_state_t *b)
{
    return same_but_level(a, b) && a->applied_percent == b->applied_percent;
}

void host_notify_init(host_notify_t *n, const host_notify_state_t *now, uint16_t level_period_ms)
{
    *n = (host_notify_t){
        .seen = *now,
        .sent = *now,
        .source = CHANGE_SOURCE_BOOT,
        .level_period_ms = level_period_ms,
    };
}

void host_notify_observe(host_notify_t *n, const host_notify_state_t *now,
                         change_source_t lamp_source)
{
    if (now->link != USB_LINK_CONFIGURED) {
        n->telemetry_period_ms = 0;
    }
    if (!lamp_state_equal(&now->lamp, &n->seen.lamp)) {
        n->source = lamp_source;
    } else if (now->link != n->seen.link) {
        n->source = CHANGE_SOURCE_USB;
    } else if (now->applied_percent != n->seen.applied_percent) {
        n->source = CHANGE_SOURCE_AUTO_BRIGHTNESS;
    } else if (!same_xy(now->xy, n->seen.xy)) {
        n->source = CHANGE_SOURCE_HOST; /* only the host redefines a color */
    }
    n->seen = *now;
}

bool host_notify_state_due(const host_notify_t *n, uint32_t now_ms)
{
    if (same_content(&n->seen, &n->sent)) {
        return false;
    }
    return !same_but_level(&n->seen, &n->sent) ||
           time_elapsed_ms(now_ms, n->level_sent_ms) >= n->level_period_ms;
}

proto_state_t host_notify_state(const host_notify_t *n)
{
    const bool new_content = !same_content(&n->seen, &n->sent);
    const proto_state_t state = {
        .state_seq = new_content ? (uint16_t)(n->state_seq + 1u) : n->state_seq,
        .lamp = n->seen.lamp,
        .applied_percent = n->seen.applied_percent,
        .link = n->seen.link,
        .source = n->source,
        .xy = n->seen.xy,
    };
    return state;
}

void host_notify_state_sent(host_notify_t *n, uint32_t now_ms)
{
    if (!same_content(&n->seen, &n->sent)) {
        n->state_seq++;
        if (same_but_level(&n->seen, &n->sent)) {
            n->level_sent_ms = now_ms;
        }
    }
    n->sent = n->seen;
}

void host_notify_set_telemetry(host_notify_t *n, uint16_t period_ms, uint32_t now_ms)
{
    n->telemetry_period_ms = period_ms;
    n->telemetry_next_ms = now_ms;
}

bool host_notify_take_telemetry(host_notify_t *n, uint32_t now_ms)
{
    if (n->telemetry_period_ms == 0u || !time_reached(now_ms, n->telemetry_next_ms)) {
        return false;
    }
    n->telemetry_next_ms = now_ms + n->telemetry_period_ms;
    return true;
}
