/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 */
#include <stddef.h>
#include <string.h>

#include "bl_core.h"

/* The page to program, filled by DATA; FINISH reuses it for the record. */
static uint32_t s_page[BL_PAGE_SIZE / sizeof(uint32_t)];
static uint8_t s_reasons;
static bool s_updating;

static uint32_t get32(const uint8_t *p)
{
    return (uint32_t)p[0] | ((uint32_t)p[1] << 8) | ((uint32_t)p[2] << 16) | ((uint32_t)p[3] << 24);
}

static void put32(uint8_t *p, uint32_t value)
{
    for (size_t i = 0; i < 4u; i++) {
        p[i] = (uint8_t)(value >> (8u * i));
    }
}

static bl_record_t read_record(const uint8_t *flash)
{
    bl_record_t record;
    memcpy(&record, &flash[BL_RECORD_OFFSET], sizeof record);
    return record;
}

bool bl_app_valid(const uint8_t *flash)
{
    const bl_record_t record = read_record(flash);
    return record.magic == BL_RECORD_MAGIC && record.size != 0u && record.size <= BL_APP_MAX &&
           bl_crc32(&flash[BL_APP_OFFSET], record.size) == record.crc;
}

uint8_t bl_boot_reasons(const uint8_t *flash, bool requested)
{
    s_updating = false;
    s_reasons = (uint8_t)((requested ? BL_REASON_REQUESTED : 0u) |
                          (bl_app_valid(flash) ? 0u : BL_REASON_APP_INVALID));
    return s_reasons;
}

bool bl_proto_updating(void)
{
    return s_updating;
}

/* INFO body, from BL_REPLY_OFFSET. The record shows as zeros when absent. */
static void info(const uint8_t *flash, uint8_t *body)
{
    body[0] = BL_PROTO_MAJOR;
    body[1] = BL_PROTO_MINOR;
    body[2] = s_reasons;
    put32(&body[4], BL_APP_OFFSET);
    put32(&body[8], BL_APP_MAX);
    body[12] = (uint8_t)(BL_PAGE_SIZE & 0xFFu);
    body[13] = (uint8_t)(BL_PAGE_SIZE >> 8);
    body[14] = BL_DATA_MAX;
    const bl_record_t record = read_record(flash);
    if (record.magic == BL_RECORD_MAGIC) {
        put32(&body[16], record.size);
        put32(&body[20], record.crc);
        put32(&body[24], record.tag);
    }
}

static uint8_t finish(const uint8_t *flash, uint32_t size, const uint8_t *data)
{
    if (size == 0u || size > BL_APP_MAX) {
        return BL_ERR_ARGUMENT;
    }
    const uint32_t crc = get32(&data[0]);
    if (bl_crc32(&flash[BL_APP_OFFSET], size) != crc) {
        return BL_ERR_CRC;
    }
    memset(s_page, 0xFF, sizeof s_page);
    s_page[0] = BL_RECORD_MAGIC;
    s_page[1] = size;
    s_page[2] = crc;
    s_page[3] = get32(&data[4]);
    if (!bl_port_write_page(BL_RECORD_OFFSET, s_page)) {
        return BL_ERR_FLASH; /* still updating: FINISH may be sent again */
    }
    s_updating = false;
    return BL_OK;
}

bool bl_proto_handle(const uint8_t *flash, const uint8_t request[BL_REPORT_SIZE],
                     uint8_t reply[BL_REPORT_SIZE])
{
    const uint32_t arg = get32(&request[4]);
    const uint8_t *data = &request[BL_DATA_OFFSET];
    uint8_t status = BL_OK;
    bool boot = false;

    memset(reply, 0, BL_REPORT_SIZE);
    reply[0] = request[0];
    reply[1] = request[1];

    switch (request[0]) {
        case BL_CMD_INFO:
            info(flash, &reply[BL_REPLY_OFFSET]);
            break;
        case BL_CMD_BEGIN:
            /* The record goes first: from now on, a reset or a power loss
             * leaves the bootloader in update mode. */
            s_updating = bl_port_write_page(BL_RECORD_OFFSET, NULL) &&
                         read_record(flash).magic != BL_RECORD_MAGIC;
            status = s_updating ? BL_OK : BL_ERR_FLASH;
            break;
        case BL_CMD_DATA: {
            const uint32_t length = request[2];
            if (length > BL_DATA_MAX || arg > BL_PAGE_SIZE - length) {
                status = BL_ERR_ARGUMENT;
            } else {
                memcpy((uint8_t *)s_page + arg, data, length);
            }
            break;
        }
        case BL_CMD_PROGRAM:
            if (!s_updating) {
                status = BL_ERR_SEQUENCE;
            } else if (arg >= BL_APP_PAGES) {
                status = BL_ERR_ARGUMENT;
            } else if (!bl_port_write_page(BL_APP_OFFSET + (arg * BL_PAGE_SIZE), s_page)) {
                status = BL_ERR_FLASH;
            }
            break;
        case BL_CMD_FINISH:
            status = s_updating ? finish(flash, arg, data) : BL_ERR_SEQUENCE;
            break;
        case BL_CMD_BOOT:
            boot = true;
            break;
        default:
            status = BL_ERR_COMMAND;
            break;
    }
    reply[2] = status;
    return boot;
}
