/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * What the lamp sends the host on its own, without I/O: a STATE message when
 * what it shows changes, and the TELEMETRY stream at the period the host set.
 * The application observes the lamp at every pass, sends what is due once
 * the way is clear, and says when a STATE left. Changes made meanwhile merge
 * into the next STATE, whose state_seq grows by one: a gap tells the host
 * that it lost a message.
 */
#ifndef DISLED_HOST_NOTIFY_H
#define DISLED_HOST_NOTIFY_H

#include <stdbool.h>
#include <stdint.h>

#include "disled_types.h"
#include "lamp_fsm.h"
#include "protocol.h"

/* What a STATE message describes, without its sequence number. */
typedef struct {
    lamp_state_t lamp;
    uint8_t applied_percent;
    usb_link_t link;
    rgb16_t rgb; /* changes alone when the host redefines the current color */
} host_notify_state_t;

typedef struct {
    host_notify_state_t seen; /* at the last observation */
    host_notify_state_t sent; /* in the last STATE message */
    uint16_t state_seq;       /* that of the last STATE message */
    change_source_t source;   /* of the latest change seen */
    /* A change of the applied level alone leaves at most this often. */
    uint16_t level_period_ms;
    uint32_t level_sent_ms;
    uint16_t telemetry_period_ms; /* 0: no stream */
    uint32_t telemetry_next_ms;
} host_notify_t;

/* `now`: what the lamp shows at boot, which needs no message. */
void host_notify_init(host_notify_t *n, const host_notify_state_t *now, uint16_t level_period_ms);

/* Records what the lamp shows now, `lamp_source` being the cause of the
 * latest change of the lamp state. Away from a configured link the stream
 * stops, whatever else changed: no host reads it. */
void host_notify_observe(host_notify_t *n, const host_notify_state_t *now,
                         change_source_t lamp_source);

/* New content waits since the last STATE, and a change of the level alone
 * has waited its period. */
bool host_notify_state_due(const host_notify_t *n, uint32_t now_ms);

/* The STATE to send now, as an answer or on its own: the content seen, with
 * the state_seq it takes. */
proto_state_t host_notify_state(const host_notify_t *n);

/* The STATE of host_notify_state() left at `now_ms`. A send that failed
 * takes no number. */
void host_notify_state_sent(host_notify_t *n, uint32_t now_ms);

/* A period of 0 stops the stream; the first sample is due at once. */
void host_notify_set_telemetry(host_notify_t *n, uint16_t period_ms, uint32_t now_ms);

/* True when a TELEMETRY sample is due at `now_ms`; the next one is then due a
 * period later. */
bool host_notify_take_telemetry(host_notify_t *n, uint32_t now_ms);

#endif /* DISLED_HOST_NOTIFY_H */
