/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * Line-based command console on the CDC port, debug builds only. It reads and
 * dispatches the lines, and the application provides the commands.
 */
#ifndef DISLED_CONSOLE_H
#define DISLED_CONSOLE_H

#if DISLED_DEBUG

#include <stddef.h>

typedef struct {
    const char *name;
    const char *usage; /* arguments, for "help" */
    const char *help;
    void (*handler)(int argc, char **argv);
} console_command_t;

void console_init(const console_command_t *commands, size_t count);

/* Reads pending input and runs complete lines. */
void console_task(void);

__attribute__((format(printf, 1, 2))) void console_printf(const char *format, ...);

#endif

#endif /* DISLED_CONSOLE_H */
