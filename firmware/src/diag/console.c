/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include "console.h"

#if DISLED_DEBUG

#include <stdarg.h>
#include <stdbool.h>
#include <string.h>

#include "fmt.h"
#include "usb/usb_device.h"

#define LINE_MAX 64u
#define ARGS_MAX 6

static const console_command_t *s_commands;
static size_t s_count;
static char s_line[LINE_MAX];
static size_t s_length;

void console_init(const console_command_t *commands, size_t count)
{
    s_commands = commands;
    s_count = count;
    s_length = 0;
}

void console_printf(const char *format, ...)
{
    char text[128];
    va_list args;
    va_start(args, format);
    size_t length = fmt_vsnprintf(text, sizeof text, format, args);
    va_end(args);
    if (length >= sizeof text) {
        length = sizeof text - 1u;
    }
    usb_cdc_write(text, length);
}

static void print_help(void)
{
    console_printf("commands:\r\n");
    for (size_t i = 0; i < s_count; i++) {
        console_printf("  %-6s %-16s %s\r\n", s_commands[i].name, s_commands[i].usage,
                       s_commands[i].help);
    }
}

static bool is_space(char c)
{
    return c == ' ' || c == '\t';
}

/* Splits the line in place on spaces (strtok would pull in newlib's
 * reentrancy state). */
static int split(char *line, char *argv[ARGS_MAX])
{
    int argc = 0;
    char *p = line;
    while (*p != '\0' && argc < ARGS_MAX) {
        while (is_space(*p)) {
            *p++ = '\0';
        }
        if (*p == '\0') {
            break;
        }
        argv[argc++] = p;
        while (*p != '\0' && !is_space(*p)) {
            p++;
        }
    }
    return argc;
}

static void execute(char *line)
{
    char *argv[ARGS_MAX];
    const int argc = split(line, argv);
    if (argc == 0) {
        return;
    }
    if (strcmp(argv[0], "help") == 0) {
        print_help();
        return;
    }
    for (size_t i = 0; i < s_count; i++) {
        if (strcmp(argv[0], s_commands[i].name) == 0) {
            s_commands[i].handler(argc, argv);
            return;
        }
    }
    console_printf("unknown command '%s', try 'help'\r\n", argv[0]);
}

void console_task(void)
{
    char input[32];
    const size_t received = usb_cdc_read(input, sizeof input);
    for (size_t i = 0; i < received; i++) {
        const char c = input[i];
        if (c == '\r' || c == '\n') {
            if (s_length > 0u) {
                usb_cdc_write("\r\n", 2u);
                s_line[s_length] = '\0';
                execute(s_line);
                s_length = 0;
                usb_cdc_write("> ", 2u);
            }
        } else if (c == '\b' || c == 0x7F) {
            if (s_length > 0u) {
                s_length--;
                usb_cdc_write("\b \b", 3u);
            }
        } else if (s_length < LINE_MAX - 1u && c >= ' ') {
            s_line[s_length++] = c;
            usb_cdc_write(&c, 1u);
        }
    }
}

#endif
