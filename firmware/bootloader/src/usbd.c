/*
 * SPDX-License-Identifier: GPL-3.0-or-later
 * SPDX-FileCopyrightText: 2026 The Disled contributors
 * Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
 *
 * USBD is the STM32F103 USB device ("FSDEV"): 16-bit registers at 32-bit
 * steps from 0x40005C00, and 512 bytes of packet memory (PMA) at 0x40006000,
 * which the CPU sees as 16-bit words at 32-bit steps. TinyUSB drives it in
 * the application (CFG_TUD_WCH_USBIP_FSDEV).
 *
 * Polled, without interrupts. Endpoint 0: control, 64 bytes; every
 * descriptor fits one packet, so a control read is one IN packet, then the
 * status OUT. Endpoint 1: interrupt IN and OUT, 64 bytes, one reply for
 * each request. OUT is armed again only once the host has collected the
 * reply: a reply is never overwritten, and the host waits (NAK) while the
 * flash is written.
 *
 * Erratum handled by TinyUSB: on this chip, endpoint 0 may acknowledge an
 * OUT packet while its STAT_RX is NAK. This driver never expects OUT data on
 * endpoint 0 (it stalls control writes that have a data stage), and arms the
 * status OUT together with each IN packet, so an early ACK does no harm.
 */
#include "usbd.h"

#include <stddef.h>
#include <stdint.h>

#include "bl_core.h"
#include "ch32.h"

#define USBD_BASE 0x40005C00u
#define PMA_BASE  0x40006000u

#define EPR(ep)   (*(volatile uint16_t *)(USBD_BASE + (4u * (ep))))
#define CNTR      (*(volatile uint16_t *)(USBD_BASE + 0x40u))
#define ISTR      (*(volatile uint16_t *)(USBD_BASE + 0x44u))
#define DADDR     (*(volatile uint16_t *)(USBD_BASE + 0x4Cu))
#define BTABLE    (*(volatile uint16_t *)(USBD_BASE + 0x50u))
#define PMA(addr) (*(volatile uint16_t *)(PMA_BASE + (2u * (addr))))

/* EPnR: CTR_* clear on writing 0, DTOG_* and STAT_* toggle on writing 1. */
#define EP_CTR_RX    0x8000u
#define EP_DTOG_RX   0x4000u
#define EP_STAT_RX   0x3000u
#define EP_SETUP     0x0800u
#define EP_CONFIG    0x070Fu /* type, kind, address */
#define EP_CTR_TX    0x0080u
#define EP_DTOG_TX   0x0040u
#define EP_STAT_TX   0x0030u
#define EP_CONTROL   0x0200u
#define EP_INTERRUPT 0x0600u
#define RX_STALL     0x1000u
#define RX_VALID     0x3000u
#define TX_STALL     0x0010u
#define TX_NAK       0x0020u
#define TX_VALID     0x0030u

#define CNTR_FRES  0x0001u
#define ISTR_CTR   0x8000u
#define ISTR_WKUP  0x1000u
#define ISTR_SUSP  0x0800u
#define ISTR_RESET 0x0400u
#define ISTR_EP_ID 0x000Fu
#define DADDR_EF   0x0080u

/* Packet memory: the buffer table (TX address, TX count, RX address, RX
 * count for each endpoint), then four 64-byte buffers. */
#define BT(ep, field) ((8u * (ep)) + (2u * (field)))
#define BT_TX_ADDR    0u
#define BT_TX_COUNT   1u
#define BT_RX_ADDR    2u
#define BT_RX_COUNT   3u
#define BUF_EP0_RX    0x40u
#define BUF_EP0_TX    0x80u
#define BUF_EP1_RX    0xC0u
#define BUF_EP1_TX    0x100u
#define RX_SIZE_64    0x8400u /* two blocks of 32 bytes */

#define EP1          1u
#define EP0_SIZE     64u
#define ESIG_UNIID   ((const volatile uint32_t *)0x1FFFF7E8u)
#define SERIAL_CHARS 24u

/* clang-format off */
static const uint8_t desc_device[] = {
    18, 0x01,                     /* bLength, DEVICE */
    0x00, 0x02,                   /* USB 2.0 */
    0x00, 0x00, 0x00,             /* class, subclass, protocol: in the interface */
    EP0_SIZE,
    (uint8_t)(USBD_VID & 0xFFu), (uint8_t)(USBD_VID >> 8),
    (uint8_t)(USBD_PID & 0xFFu), (uint8_t)(USBD_PID >> 8),
    (uint8_t)(USBD_BCD_DEVICE & 0xFFu), (uint8_t)(USBD_BCD_DEVICE >> 8),
    1, 2, 3,                      /* strings: manufacturer, product, serial number */
    1,                            /* configurations */
};

/* Vendor page 0xFF00, 64-byte reports without report ID: requests (OUT)
 * and replies (IN). */
static const uint8_t desc_report[] = {
    0x06, 0x00, 0xFF,             /* Usage Page (vendor 0xFF00) */
    0x09, 0x01,                   /* Usage (1) */
    0xA1, 0x01,                   /* Collection (Application) */
    0x15, 0x00,                   /*   Logical Minimum (0) */
    0x26, 0xFF, 0x00,             /*   Logical Maximum (255) */
    0x75, 0x08,                   /*   Report Size (8) */
    0x95, 0x40,                   /*   Report Count (64) */
    0x09, 0x02,                   /*   Usage (2) */
    0x91, 0x02,                   /*   Output (Data, Variable, Absolute) */
    0x09, 0x03,                   /*   Usage (3) */
    0x81, 0x02,                   /*   Input (Data, Variable, Absolute) */
    0xC0,                         /* End Collection */
};

#define CONFIG_LEN 41u
#define HID_OFFSET 18u
static const uint8_t desc_config[CONFIG_LEN] = {
    /* Configuration: one interface, bus powered, 100 mA (LED and MCU) */
    9, 0x02, CONFIG_LEN, 0, 1, 1, 0, 0x80, 50,
    /* Interface 0: HID, two endpoints, no boot protocol */
    9, 0x04, 0, 0, 2, 0x03, 0x00, 0x00, 0,
    /* HID 1.11, one report descriptor */
    9, 0x21, 0x11, 0x01, 0, 1, 0x22, (uint8_t)sizeof desc_report, 0,
    /* Endpoint 1 IN, then OUT: interrupt, 64 bytes, every 1 ms */
    7, 0x05, 0x81, 0x03, BL_REPORT_SIZE, 0, 1,
    7, 0x05, 0x01, 0x03, BL_REPORT_SIZE, 0, 1,
};
/* clang-format on */

_Static_assert(sizeof desc_device <= EP0_SIZE && sizeof desc_report <= EP0_SIZE,
               "a descriptor must fit one packet");

/* The same serial number as the application: the host can follow the lamp
 * across the restarts of an update. */
static char s_serial[SERIAL_CHARS + 1u];
static const char *const strings[] = { "Disled", "Disled bootloader", s_serial };

static uint8_t s_address; /* DADDR_EF | address, set after the status stage */
static uint8_t s_config;
static bool s_suspended;
static bool s_boot;
static uint8_t s_request[BL_REPORT_SIZE];
static uint8_t s_reply[BL_REPORT_SIZE];

/* Sets the STAT bits of `mask` to `stat`, keeping the endpoint type and
 * address, its data toggles and its CTR flags. */
static void ep_set_stat(uint32_t ep, uint32_t mask, uint32_t stat)
{
    const uint32_t reg = EPR(ep);
    EPR(ep) = (uint16_t)(((reg ^ stat) & mask) | (reg & EP_CONFIG) | EP_CTR_RX | EP_CTR_TX);
}

static void ep_clear_ctr(uint32_t ep, uint32_t ctr)
{
    const uint32_t reg = EPR(ep);
    EPR(ep) = (uint16_t)((reg & EP_CONFIG) | ((EP_CTR_RX | EP_CTR_TX) & ~ctr));
}

/* Type and address `config`, both STAT fields to `stat`, both data toggles
 * to DATA0, CTR flags cleared. */
static void ep_init(uint32_t ep, uint32_t config, uint32_t stat)
{
    const uint32_t reg = EPR(ep);
    EPR(ep) = (uint16_t)(((reg ^ stat) & (EP_STAT_RX | EP_STAT_TX)) |
                         (reg & (EP_DTOG_RX | EP_DTOG_TX)) | config);
}

static void ep1_init(void)
{
    ep_init(EP1, EP_INTERRUPT | EP1, RX_VALID | TX_NAK);
}

static void pma_write(uint32_t addr, const uint8_t *src, uint32_t length)
{
    for (uint32_t i = 0; i < length; i += 2u) {
        const uint32_t high = (i + 1u < length) ? src[i + 1u] : 0u;
        PMA(addr + i) = (uint16_t)(src[i] | (high << 8));
    }
}

static void pma_read(uint32_t addr, uint8_t *dst, uint32_t length)
{
    for (uint32_t i = 0; i < length; i += 2u) {
        const uint32_t word = PMA(addr + i);
        dst[i] = (uint8_t)word;
        dst[i + 1u] = (uint8_t)(word >> 8);
    }
}

/* String descriptors, in UTF-16 straight into the IN buffer. */
static int32_t write_string(uint32_t index)
{
    if (index == 0u) {
        PMA(BUF_EP0_TX) = 0x0304u;      /* 4 bytes, STRING */
        PMA(BUF_EP0_TX + 2u) = 0x0409u; /* English (United States) */
        return 4;
    }
    if (index > sizeof strings / sizeof strings[0]) {
        return -1;
    }
    const char *text = strings[index - 1u];
    uint32_t length = 2u;
    while (*text != '\0') {
        PMA(BUF_EP0_TX + length) = (uint8_t)*text++;
        length += 2u;
    }
    PMA(BUF_EP0_TX) = (uint16_t)(0x0300u | length);
    return (int32_t)length;
}

/* Writes the descriptor into the IN buffer of endpoint 0. Returns its
 * length, or -1 if there is no such descriptor. */
static int32_t write_descriptor(uint32_t value)
{
    const uint8_t *src;
    uint32_t length;
    switch (value >> 8) {
        case 0x01:
            src = desc_device;
            length = sizeof desc_device;
            break;
        case 0x02:
            src = desc_config;
            length = sizeof desc_config;
            break;
        case 0x03:
            return write_string(value & 0xFFu);
        case 0x21: /* HID, within the configuration */
            src = &desc_config[HID_OFFSET];
            length = 9u;
            break;
        case 0x22:
            src = desc_report;
            length = sizeof desc_report;
            break;
        default: /* device qualifier included: a full-speed device stalls */
            return -1;
    }
    pma_write(BUF_EP0_TX, src, length);
    return (int32_t)length;
}

static void ep0_setup(void)
{
    uint8_t setup[8];
    pma_read(BUF_EP0_RX, setup, sizeof setup);
    ep_clear_ctr(0, EP_CTR_RX);

    const uint32_t request = ((uint32_t)setup[1] << 8) | setup[0]; /* bRequest, bmRequestType */
    const uint32_t value = ((uint32_t)setup[3] << 8) | setup[2];
    const uint32_t length = ((uint32_t)setup[7] << 8) | setup[6];
    int32_t size = 0;

    switch (request) {
        case 0x0680: /* GET_DESCRIPTOR, device */
        case 0x0681: /* GET_DESCRIPTOR, interface: HID and report */
            size = write_descriptor(value);
            break;
        case 0x0500: /* SET_ADDRESS, applied once the status stage is over */
            s_address = (uint8_t)(DADDR_EF | (value & 0x7Fu));
            break;
        case 0x0900: /* SET_CONFIGURATION: data toggles back to DATA0 */
            s_config = (uint8_t)value;
            ep1_init();
            break;
        case 0x0880: /* GET_CONFIGURATION */
            PMA(BUF_EP0_TX) = s_config;
            size = 1;
            break;
        case 0x0080: /* GET_STATUS: bus powered, no remote wake-up, no halt */
        case 0x0081:
        case 0x0082:
            PMA(BUF_EP0_TX) = 0u;
            size = 2;
            break;
        case 0x0102: /* CLEAR_FEATURE (endpoint halt): the toggle to DATA0 */
            if ((setup[4] & 0x7Fu) == EP1) {
                ep1_init();
            }
            break;
        default:
            /* Other requests without data (SET_IDLE, SET_INTERFACE...) are
             * accepted and ignored; the others stall, data stage or not. */
            if ((setup[0] & 0x80u) != 0u || length != 0u) {
                size = -1;
            }
            break;
    }

    if (size < 0) {
        ep_set_stat(0, EP_STAT_RX | EP_STAT_TX, RX_STALL | TX_STALL);
        return;
    }
    /* IN data, or the status packet: one IN packet, and the status OUT. */
    PMA(BT(0, BT_TX_COUNT)) = (uint16_t)(((uint32_t)size < length) ? (uint32_t)size : length);
    ep_set_stat(0, EP_STAT_RX | EP_STAT_TX, RX_VALID | TX_VALID);
}

static void ep0_event(uint32_t reg)
{
    if ((reg & EP_CTR_RX) != 0u) {
        if ((reg & EP_SETUP) != 0u) {
            ep0_setup();
        } else {
            ep_clear_ctr(0, EP_CTR_RX); /* status OUT of a control read */
        }
    }
    if ((reg & EP_CTR_TX) != 0u) {
        ep_clear_ctr(0, EP_CTR_TX);
        if (s_address != 0u) {
            DADDR = s_address;
            s_address = 0;
        }
    }
}

/* True when it handled a request. */
static bool ep1_event(uint32_t reg)
{
    const bool request = (reg & EP_CTR_RX) != 0u;
    if (request) {
        pma_read(BUF_EP1_RX, s_request, BL_REPORT_SIZE);
        ep_clear_ctr(EP1, EP_CTR_RX);
        if (bl_proto_handle(BL_FLASH, s_request, s_reply)) {
            s_boot = true;
        }
        pma_write(BUF_EP1_TX, s_reply, BL_REPORT_SIZE);
        PMA(BT(EP1, BT_TX_COUNT)) = BL_REPORT_SIZE;
        ep_set_stat(EP1, EP_STAT_TX, TX_VALID);
    }
    if ((reg & EP_CTR_TX) != 0u) {
        ep_clear_ctr(EP1, EP_CTR_TX);
        if (s_boot) {
            bl_system_reset();
        }
        ep_set_stat(EP1, EP_STAT_RX, RX_VALID);
    }
    return request;
}

static void bus_reset(void)
{
    BTABLE = 0;
    PMA(BT(0, BT_TX_ADDR)) = BUF_EP0_TX;
    PMA(BT(0, BT_RX_ADDR)) = BUF_EP0_RX;
    PMA(BT(0, BT_RX_COUNT)) = RX_SIZE_64;
    PMA(BT(EP1, BT_TX_ADDR)) = BUF_EP1_TX;
    PMA(BT(EP1, BT_RX_ADDR)) = BUF_EP1_RX;
    PMA(BT(EP1, BT_RX_COUNT)) = RX_SIZE_64;
    ep_init(0, EP_CONTROL, RX_VALID | TX_NAK);
    s_address = 0;
    s_config = 0;
    s_suspended = false;
    DADDR = DADDR_EF; /* address 0 */
}

void usbd_init(void)
{
    for (uint32_t i = 0; i < SERIAL_CHARS; i++) {
        const uint32_t word = ESIG_UNIID[i / 8u];
        s_serial[i] = "0123456789ABCDEF"[(word >> (28u - (4u * (i % 8u)))) & 0x0Fu];
    }

    RCC->APB1PCENR |= RCC_USBEN;
    CNTR = CNTR_FRES; /* transceiver on (PDWN cleared), logic still in reset */
    for (volatile uint32_t i = 0; i < 64u; i++) {
        /* t_STARTUP, 1 us */
    }
    CNTR = 0;
    ISTR = 0;
    EXTEN->EXTEN_CTR |= EXTEN_USBD_PU_EN; /* D+ pull-up: the host sees the device */
}

bool usbd_poll(void)
{
    const uint32_t istr = ISTR;
    if ((istr & ISTR_RESET) != 0u) {
        ISTR = (uint16_t)~ISTR_RESET;
        bus_reset();
        return false;
    }
    if ((istr & ISTR_SUSP) != 0u) {
        ISTR = (uint16_t)~ISTR_SUSP;
        s_suspended = s_config != 0u; /* not on a charger, before any host */
    }
    if ((istr & ISTR_WKUP) != 0u) {
        ISTR = (uint16_t)~ISTR_WKUP;
        s_suspended = false;
    }
    if ((istr & ISTR_CTR) != 0u) {
        const uint32_t ep = istr & ISTR_EP_ID;
        const uint32_t reg = EPR(ep);
        if (ep == 0u) {
            ep0_event(reg);
        } else if (ep == EP1) {
            return ep1_event(reg);
        }
    }
    return false;
}

bool usbd_suspended(void)
{
    return s_suspended;
}

bool usbd_booting(void)
{
    return s_boot;
}
