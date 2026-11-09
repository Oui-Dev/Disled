/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The DISLED contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * USB descriptors: one vendor HID interface for the protocol, plus a CDC
 * console in debug builds. TinyUSB pulls them through its callbacks.
 */
#include "usb_descriptors.h"

#include <stddef.h>
#include <stdint.h>

#include "bsp/bsp_uid.h"
#include "protocol.h"
#include "tusb.h"

/* bcdDevice = firmware MAJOR.MINOR in BCD (two digits each). */
#define TO_BCD2(value) ((uint16_t)((((value) / 10u) % 10u) << 4 | ((value) % 10u)))
#define USB_BCD_DEVICE \
    ((uint16_t)(TO_BCD2(DISLED_FW_VERSION_MAJOR) << 8 | TO_BCD2(DISLED_FW_VERSION_MINOR)))

enum {
    STRID_LANGID = 0,
    STRID_MANUFACTURER,
    STRID_PRODUCT,
    STRID_SERIAL,
    STRID_HID,
#if CFG_TUD_CDC
    STRID_CDC,
#endif
};

static const tusb_desc_device_t desc_device = {
    .bLength = sizeof(tusb_desc_device_t),
    .bDescriptorType = TUSB_DESC_DEVICE,
    .bcdUSB = 0x0200,
#if CFG_TUD_CDC
    /* Composite device with an interface association (CDC needs one). */
    .bDeviceClass = TUSB_CLASS_MISC,
    .bDeviceSubClass = MISC_SUBCLASS_COMMON,
    .bDeviceProtocol = MISC_PROTOCOL_IAD,
#else
    .bDeviceClass = 0x00,
    .bDeviceSubClass = 0x00,
    .bDeviceProtocol = 0x00,
#endif
    .bMaxPacketSize0 = CFG_TUD_ENDPOINT0_SIZE,
    .idVendor = USB_VID,
    .idProduct = USB_PID,
    .bcdDevice = USB_BCD_DEVICE,
    .iManufacturer = STRID_MANUFACTURER,
    .iProduct = STRID_PRODUCT,
    .iSerialNumber = STRID_SERIAL,
    .bNumConfigurations = 1,
};

/* Vendor page 0xFF00: one command report (OUT) and one message report (IN). */
static const uint8_t desc_hid_report[] = {
    HID_USAGE_PAGE_N(HID_USAGE_PAGE_VENDOR, 2),
    HID_USAGE(0x01),
    HID_COLLECTION(HID_COLLECTION_APPLICATION),
    HID_REPORT_ID(PROTO_REPORT_ID_COMMAND) HID_USAGE(0x02),
    HID_LOGICAL_MIN(0x00),
    HID_LOGICAL_MAX_N(0xFF, 2),
    HID_REPORT_SIZE(8),
    HID_REPORT_COUNT(PROTO_REPORT_SIZE),
    HID_OUTPUT(HID_DATA | HID_VARIABLE | HID_ABSOLUTE),
    HID_REPORT_ID(PROTO_REPORT_ID_MESSAGE) HID_USAGE(0x03),
    HID_LOGICAL_MIN(0x00),
    HID_LOGICAL_MAX_N(0xFF, 2),
    HID_REPORT_SIZE(8),
    HID_REPORT_COUNT(PROTO_REPORT_SIZE),
    HID_INPUT(HID_DATA | HID_VARIABLE | HID_ABSOLUTE),
    HID_COLLECTION_END,
};

#define CONFIG_TOTAL_LEN \
    (TUD_CONFIG_DESC_LEN + TUD_HID_INOUT_DESC_LEN + (CFG_TUD_CDC ? TUD_CDC_DESC_LEN : 0))

static const uint8_t desc_configuration[] = {
    TUD_CONFIG_DESCRIPTOR(1, USB_ITF_COUNT, 0, CONFIG_TOTAL_LEN, 0x00, USB_MAX_POWER_MA),
    TUD_HID_INOUT_DESCRIPTOR(USB_ITF_HID, STRID_HID, HID_ITF_PROTOCOL_NONE, sizeof(desc_hid_report),
                             USB_EP_HID_OUT, USB_EP_HID_IN, CFG_TUD_HID_EP_BUFSIZE,
                             USB_HID_POLL_INTERVAL_MS),
#if CFG_TUD_CDC
    TUD_CDC_DESCRIPTOR(USB_ITF_CDC_COMM, STRID_CDC, USB_EP_CDC_NOTIF, 8, USB_EP_CDC_OUT,
                       USB_EP_CDC_IN, CFG_TUD_CDC_EP_BUFSIZE),
#endif
};

_Static_assert(sizeof(desc_configuration) == CONFIG_TOTAL_LEN, "configuration descriptor size");
_Static_assert(PROTO_REPORT_SIZE + 1u <= CFG_TUD_HID_EP_BUFSIZE, "HID report must fit one packet");

static const char *const string_table[] = {
    [STRID_MANUFACTURER] = USB_MANUFACTURER_STRING,
    [STRID_PRODUCT] = USB_PRODUCT_STRING,
    [STRID_HID] = "Disled control",
#if CFG_TUD_CDC
    [STRID_CDC] = "Disled console",
#endif
};

/* Longest string: the 24-digit serial number. */
#define STRING_MAX_CHARS BSP_UID_HEX_CHARS
static uint16_t desc_string[1u + STRING_MAX_CHARS];

const uint8_t *tud_descriptor_device_cb(void)
{
    return (const uint8_t *)&desc_device;
}

const uint8_t *tud_hid_descriptor_report_cb(uint8_t instance)
{
    (void)instance;
    return desc_hid_report;
}

const uint8_t *tud_descriptor_configuration_cb(uint8_t index)
{
    (void)index;
    return desc_configuration;
}

const uint16_t *tud_descriptor_string_cb(uint8_t index, uint16_t langid)
{
    (void)langid;
    size_t count = 0;

    if (index == STRID_LANGID) {
        desc_string[1] = 0x0409; /* English (United States) */
        count = 1;
    } else if (index == STRID_SERIAL) {
        char serial[BSP_UID_HEX_CHARS];
        bsp_uid_hex(serial);
        for (count = 0; count < BSP_UID_HEX_CHARS; count++) {
            desc_string[1u + count] = (uint16_t)serial[count];
        }
    } else {
        if (index >= TU_ARRAY_SIZE(string_table) || string_table[index] == NULL) {
            return NULL;
        }
        const char *text = string_table[index];
        while (count < STRING_MAX_CHARS && text[count] != '\0') {
            desc_string[1u + count] = (uint16_t)text[count];
            count++;
        }
    }

    /* First word: total length in bytes and descriptor type. */
    desc_string[0] = (uint16_t)((TUSB_DESC_STRING << 8) | (2u * count + 2u));
    return desc_string;
}
