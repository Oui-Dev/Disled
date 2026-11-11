# Disled USB protocol, version 2.0

The lamp is a USB HID device with one vendor-defined interface. The host
sends **commands**; the lamp answers with **messages** and also sends
messages on its own (state changes, telemetry, the end of an alert). The
lamp owns the lamp state: the host reads it, changes it and follows its
notifications.

> **Status.** Version 2.0 is **frozen**: firmware 2.0.0 speaks it. Only the
> compatible changes listed in [Compatibility](#compatibility) remain
> allowed, and anything else needs version 3. Version 2 adds custom colors,
> a touch cycle, a power-only touch mode, alerts and a touch sensitivity
> setting. The lamp keeps none of them in flash: the host sends them again
> after each connection ([Connection](#connection)). A lamp with firmware
> 1.0.0 speaks version 1.1 ([Version 1 lamps](#version-1-lamps)).

## Transport

| Item | Value |
|---|---|
| USB IDs | `1209:0001` (release builds, until pid.codes grants `1209:D1ED`), `1209:0002` (debug builds), pid.codes test PIDs ([LICENSING.md](../LICENSING.md#usb-identifiers)) |
| Interface | HID, usage page `0xFF00` (vendor), usage `0x01`, named "Disled control" |
| Command report | Output report, ID `1`, 63 bytes |
| Message report | Input report, ID `2`, 63 bytes |
| Endpoints | Interrupt IN and OUT, 64 bytes, polled every 4 ms |

Hosts write commands as output reports (interrupt OUT or SET_REPORT) and
read messages as input reports. With hidapi, a command is written as
`[0x01, payload[0..63]]` and a message reads as `[0x02, payload[0..63]]`.
Hosts find the lamp by its USB IDs and usage page, not by its strings.

## Frame

Every payload (the 63 bytes after the report ID) has the same layout:

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `type` |
| 1 | 1 | `seq`: chosen by the host in a command, echoed in the reply; `0` in unsolicited messages, except `ALERT_DONE` |
| 2 | 1 | `length` of the body, 0 to 60 |
| 3 | `length` | body |
| 3 + `length` | rest | zero |

Multi-byte numbers are little-endian; signed numbers are two's complement.
Booleans are `0` or `1`.

## Commands (host to lamp)

| Type | Name | Body | Reply | Errors |
|---|---|---|---|---|
| `0x01` | `GET_INFO` | none | `INFO` | |
| `0x02` | `GET_STATE` | none | `STATE` | |
| `0x03` | `SET_STATE` | 6 bytes, see below | `STATE` (the new state) | 2, 3, 4 |
| `0x04` | `SET_TELEMETRY` | `u16 period_ms`: 0 stops, otherwise 20 or more | `ACK` | 2, 3 |
| `0x05` | `RESET` | none | `ACK`, then the lamp restarts after 50 ms | |
| `0x06` | `SET_AUTO_BIAS` | `i8 bias`, see below | `ACK` | 2, 3, 4 |
| `0x07` | `GET_DIAGNOSTICS` | none | `DIAGNOSTICS` | |
| `0x08` | `SELF_TEST` | none | `ACK`, then `SELF_TEST_RESULT` when the test ends | 4 |
| `0x09` | reserved | | | 1 |
| `0x0A` | `SET_COLOR` | 7 bytes, see below | `ACK` | 2, 3, 4 |
| `0x0B` | `GET_COLOR` | `u8 slot`, 0 to 15 | `COLOR` | 2, 3 |
| `0x0C` | `SET_TOUCH_CYCLE` | 1 + n bytes, see below | `ACK` | 2, 3 |
| `0x0D` | `GET_TOUCH_CYCLE` | none | `TOUCH_CYCLE` | |
| `0x0E` | `ALERT` | 3 bytes, see below | `ACK`, then `ALERT_DONE` when the alert ends | 2, 3, 4, 5 |
| `0x0F` | reserved | | | 1 |
| `0x10` | `SET_TOUCH_SENSITIVITY` | `i8 percent`, see below | `ACK` | 2, 3 |

A command that cannot be carried out gets an `ERROR` reply and changes
nothing. Errors: 1 unknown command, 2 bad length (a body shorter than the
command needs, or a `length` over 60), 3 bad value, 4 busy (during a
self-test), 5 lamp off.

Every command, even a refused one, tells the lamp that the host is there
([Touch modes](#touch-modes)).

`0x09` was `ENTER_BOOTLOADER` in version 1.1, which no lamp announced: the
CH32V203 starts its ROM bootloader only from a reset with BOOT held, so
the lamp has nothing to do for an update. `0x09` and `0x0F` stay reserved,
and a lamp answers them `ERROR` (unknown command).

### SET_STATE body (6 bytes)

| Offset | Field | Values |
|---|---|---|
| 0 | `fields` | bit 0 power, bit 1 color, bit 2 brightness mode, bit 3 manual level, bit 4 touch mode; other bits must be 0 |
| 1 | `on` | 0 or 1 |
| 2 | `color` | a [palette](#palette) slot that holds a color, 0 to 15 |
| 3 | `brightness_mode` | 0 automatic, 1 manual |
| 4 | `manual_level` | 1 to 100 (percent) |
| 5 | `touch_mode` | 0 normal, 1 power only |

Only the selected fields are applied, and they are applied together or not
at all. Unselected bytes are ignored, whatever their value. A slot without
a color gets `ERROR` (bad value). The 5-byte body of version 1 gets `ERROR`
(bad length).

### Palette

The lamp shows one of 16 palette slots:

- 0 to 3 hold the built-in colors, green, orange, red and warm white, a
  fixed light set by eye. They never change.
- 4 to 15 hold the custom colors that the host defines with `SET_COLOR`.
  They are empty at power-up.

A color is linear light at full brightness, with its largest channel at
65535; the brightness applies on top of it, and the LED current limit as
for any color. Hosts send a custom color as a screen shows it, in linear
light (linear sRGB, for example). The lamp balances it for its LED, so that
white shows white, and `GET_COLOR` and `STATE` give it back as sent.

### SET_COLOR body (7 bytes)

| Offset | Field | Values |
|---|---|---|
| 0 | `slot` | 4 to 15: slots 0 to 3 get `ERROR` (bad value) |
| 1 | `u16 r` | linear light at full brightness |
| 3 | `u16 g` | the same |
| 5 | `u16 b` | the same |

The largest channel must be 65535, otherwise `ERROR` (bad value). Three
zeros empty the slot instead:

- the slot leaves the touch cycle; a cycle left empty is the default one
  again (green, orange, red);
- if the lamp keeps that slot, on or off, it takes the first color of the
  cycle, with a fade and an unsolicited `STATE` (source 2);
- a `SET_STATE` that names the slot then gets `ERROR` (bad value).

Redefining the color the lamp shows fades it to the new color in 300 ms,
with an unsolicited `STATE` (source 2).

### SET_TOUCH_CYCLE body (1 + n bytes)

| Offset | Field | Values |
|---|---|---|
| 0 | `count` | 1 to 9; 0 or more than 9 gets `ERROR` (bad value) |
| 1 | `slots` | `count` slots that hold a color, none twice, otherwise `ERROR` (bad value) |

A body shorter than `1 + count` gets `ERROR` (bad length). A short press
goes to the next slot of the cycle, from the last back to the first, and
from a color outside the cycle to the first. Setting the cycle does not
change the color shown. At power-up, the cycle is 0, 1, 2 (green, orange,
red), as in version 1, where warm white also led to green.

### Touch modes

In normal mode, a short press goes to the next color of the touch cycle
and a long press (2 s) turns the lamp on or off. In power-only mode, which
suits a host that drives the color, a short press does nothing and a long
press still turns the lamp on or off. Turned off, the lamp ignores short
presses in both modes.

The lamp goes back to normal mode by itself, with an unsolicited `STATE`:

- when the USB link leaves the configured state (source 4);
- after 10 s without any command (source 5), as when the host stopped
  without a word.

A host that keeps the lamp in power-only mode sends a command at least
every 10 s, for example a `SET_STATE` that selects only the touch mode
(`03 <seq> 06 10 00 00 00 00 01`). It also brings the mode back after a
fallback. When nothing changes, the `STATE` reply keeps its `state_seq`.

### SET_AUTO_BIAS body

| Offset | Field | Values |
|---|---|---|
| 0 | `bias` (`i8`) | −50 to +50, percent |

Shifts the automatic brightness curve: the level the curve gives for the
current light is multiplied by `(100 + bias) / 100`, then kept within the
lamp's range. 0 restores the factory curve. The lamp forgets the bias when
it restarts: the host sends it again after each connection. Manual mode is
not affected. The current value is reported in `DIAGNOSTICS`.

### SET_TOUCH_SENSITIVITY body

| Offset | Field | Values |
|---|---|---|
| 0 | `percent` (`i8`) | −50 to +50; positive for a more sensitive lid |

The press, release and rise thresholds of the touch detector are
multiplied by `(100 − percent) / 100`: +50 asks for half the drop of a
finger, −50 for one and a half times it, and 0 keeps the factory
thresholds. A change during a press waits for its release. The lamp keeps
the value across a USB wake-up and forgets it when it restarts: the host
sends it again after each connection. `SET_TOUCH_SENSITIVITY` is never
busy, so the self-test judges the touch as the user set it. The current
value is reported in `DIAGNOSTICS`.

### ALERT body (3 bytes)

| Offset | Field | Values |
|---|---|---|
| 0 | `count` | 1 to 10 breaths; 0 stops the running alert |
| 1 | `u16 period_ms` | 500 to 5000, one full breath; ignored when `count` is 0 |

An alert breathes over the lamp color, up from a dim lamp and down from a
bright one, then ends by itself after its last breath. The lamp then sends
`ALERT_DONE`, with the `seq` of the `ALERT` that started the alert.

- The lamp must be on, otherwise `ERROR` (lamp off), and no alert waits for
  it. An `ALERT` with a `count` of 0 is always accepted: it stops the
  running alert (`ALERT_DONE` reason 3), and without one gets an `ACK` and
  nothing else.
- A new `ALERT` replaces the running one, without an `ALERT_DONE` for it.
- During an alert, the touch does nothing, in both touch modes: no color
  change, no power change. A press still held when the alert ends does
  nothing either, not even at its release.
- A host that turns the lamp off ends the alert (reason 1), as do a USB
  suspend (reason 2) and a self-test (reason 4).

### SELF_TEST sequence

The lamp shows red, green, blue and warm white for 1.5 s each at 50 %,
checks the light sensor, then waits up to 10 s for a touch on the lid
while it glows blue at 20 %. It then restores its state and sends
`SELF_TEST_RESULT`, with the `seq` of the `SELF_TEST` command. During the
test the lamp ignores the touch, and answers `SET_STATE`, `SET_AUTO_BIAS`,
`SET_COLOR`, `ALERT` and a second `SELF_TEST` with `ERROR` (busy). A
running alert stops when the test starts. Only a person can check the
colors: the result covers the light sensor and the touch. A USB suspend
cuts the test short: the result follows the resume, with bit 3 of
`results` clear.

## Messages (lamp to host)

| Type | Name | Sent |
|---|---|---|
| `0x81` | `INFO` | reply to `GET_INFO` |
| `0x82` | `STATE` | reply to `GET_STATE` and `SET_STATE`; unsolicited on every change |
| `0x83` | `TELEMETRY` | every `period_ms` after `SET_TELEMETRY` |
| `0x84` | `DIAGNOSTICS` | reply to `GET_DIAGNOSTICS` |
| `0x85` | `SELF_TEST_RESULT` | at the end of a self-test |
| `0x86` | `COLOR` | reply to `GET_COLOR` |
| `0x87` | `TOUCH_CYCLE` | reply to `GET_TOUCH_CYCLE` |
| `0x88` | `ALERT_DONE` | at the end of an alert |
| `0x8E` | `ACK` | reply to `SET_TELEMETRY`, `RESET`, `SET_AUTO_BIAS`, `SELF_TEST`, `SET_COLOR`, `SET_TOUCH_CYCLE`, `ALERT` and `SET_TOUCH_SENSITIVITY` |
| `0x8F` | `ERROR` | reply to a rejected command |

### INFO body (56 bytes)

The layout is that of version 1, so that a host tells the two apart by the
protocol major.

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | protocol major |
| 1 | 1 | protocol minor |
| 2 | 3 | firmware major, minor, patch |
| 5 | 1 | board revision |
| 6 | 12 | chip unique ID, most significant byte first (also the USB serial number) |
| 18 | 4 | capabilities, see below |
| 22 | 1 | reset cause: 0 unknown, 1 power-on, 2 reset pin, 3 software, 4 watchdog, 5 window watchdog, 6 low power |
| 23 | 1 | flags: bit 0 debug build, bit 1 a fault occurred before the last reset |
| 24 | 4 | fault `mcause` (0 without fault) |
| 28 | 4 | fault `mepc` (0 without fault) |
| 32 | 24 | firmware version text (`git describe`), zero padded, not terminated when 24 characters long |

Capabilities:

| Bit | Capability |
|---|---|
| 0 | telemetry |
| 1 | debug console (debug builds) |
| 2 | `SET_AUTO_BIAS` |
| 3 | `GET_DIAGNOSTICS` |
| 4 | `SELF_TEST` |
| 5 | reserved: `ENTER_BOOTLOADER` in version 1.1, never announced |
| 6 | custom colors: `SET_COLOR`, `GET_COLOR` |
| 7 | touch cycle: `SET_TOUCH_CYCLE`, `GET_TOUCH_CYCLE` |
| 8 | touch modes |
| 9 | `ALERT` |
| 10 | reserved |
| 11 | `SET_TOUCH_SENSITIVITY` |

Every version 2.0 lamp announces bits 0, 2 to 4, 6 to 9 and 11, and bit 1
in debug builds: `0x0BDD` for a release build, `0x0BDF` for a debug one.
`SET_AUTO_BIAS`, `GET_DIAGNOSTICS` and `SELF_TEST`, optional in version
1.1, are always there. Later minor versions may add bits.

### STATE body (16 bytes)

The first 9 bytes keep their place of version 1.

| Offset | Size | Field |
|---|---|---|
| 0 | 2 | `state_seq`: increases by one with every `STATE` message that carries a change; a reply without change repeats it |
| 2 | 1 | `on` |
| 3 | 1 | `color`: the palette slot, shown while on and kept while off |
| 4 | 1 | `brightness_mode` |
| 5 | 1 | `manual_level` (percent) |
| 6 | 1 | `applied_level`: brightness in use, percent (follows the light in automatic mode) |
| 7 | 1 | USB link: 0 unconfigured, 1 configured, 2 suspended |
| 8 | 1 | `source` of the latest change: 0 boot, 1 touch, 2 host, 3 automatic brightness, 4 USB, 5 host silence |
| 9 | 1 | `touch_mode` in force: 0 normal, 1 power only |
| 10 | 6 | `u16 r, g, b`: the color of the slot at full brightness, linear |

The color is there so that a host can show the lamp's color without
keeping its own copy of the palette. Unsolicited `STATE` messages caused
only by automatic brightness are sent at most every 500 ms; changes in
between are merged into the next message. So are the changes made while
earlier messages wait to leave, as when the host does not read the lamp.
Other changes, a redefined color included, leave at once. A host that sees
`state_seq` jump by more than one has missed a message and should send
`GET_STATE`.

### TELEMETRY body (32 bytes)

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | uptime, ms |
| 4 | 2 | touch: last raw TouchKey reading |
| 6 | 2 | touch: window average |
| 8 | 2 | touch: baseline |
| 10 | 2 | touch: signed drop below the baseline, per mille |
| 12 | 1 | touch state: 0 calibrating, 1 idle, 2 pressed |
| 13 | 4 | ambient light, lux x 100 |
| 17 | 2 | ambient light sensor raw count, at gain 2 and 400 ms: 0.0084 lx a count, saturated from about 550 lx |
| 19 | 6 | PWM duty red, green, blue, in whole timer ticks (3600 = 100 %; the firmware dithers to 1/16 tick and reports the rounded value) |
| 25 | 2 | estimated LED current, mA x 10 |
| 27 | 2 | LED current cap in force, mA |
| 29 | 1 | USB link |
| 30 | 2 | brightness level in use, per mille |

The touch detector calibrates for 160 ms at start-up and after each USB
wake-up: while the state is 0, the baseline and the drop read 0.
Telemetry stops when the USB link leaves the configured state.

### DIAGNOSTICS body (37 bytes)

Counters "since power-up" survive resets (they live in the backup
registers); the others restart at every boot.

| Offset | Size | Field |
|---|---|---|
| 0 | 4 | uptime, ms |
| 4 | 2 | boots since power-up |
| 6 | 2 | watchdog resets since power-up |
| 8 | 2 | faults since power-up |
| 10 | 4 | last fault `mcause` (0 without fault) |
| 14 | 4 | last fault `mepc` |
| 18 | 4 | last fault `mtval` |
| 22 | 2 | stack bytes never used since boot (the smallest margin seen) |
| 24 | 2 | I2C errors since boot |
| 26 | 2 | light sensor probes since boot (1 when it answered at the first try) |
| 28 | 2 | touch samples lost since boot (not read in time, or a measurement that never completed) |
| 30 | 2 | touch baseline recalibrations since boot (finger at start-up, object on the lid) |
| 32 | 2 | USB suspends since boot |
| 34 | 1 | automatic brightness bias, `i8` percent |
| 35 | 1 | flags: bit 0 light sensor present, bit 1 crystal running |
| 36 | 1 | touch sensitivity correction, `i8` percent |

### SELF_TEST_RESULT body (9 bytes)

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | results: bit 0 light sensor answered, bit 1 touch baseline within the expected range, bit 2 a touch was detected, bit 3 the test ran to its end |
| 1 | 4 | ambient light during the test, lux × 100 |
| 5 | 2 | touch baseline, TouchKey counts |
| 7 | 2 | largest touch drop seen while waiting, per mille (`i16`) |

### COLOR body (7 bytes)

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | slot |
| 1 | 6 | `u16 r, g, b`, as in `SET_COLOR`; three zeros for an empty slot |

### TOUCH_CYCLE body (10 bytes)

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | `count`, 1 to 9 |
| 1 | 9 | the slots of the cycle, in order; zero after `count` |

### ALERT_DONE body (1 byte)

| Offset | Size | Field |
|---|---|---|
| 0 | 1 | reason: 0 every breath shown, 1 lamp turned off, 2 USB suspend, 3 stopped by an `ALERT` with a `count` of 0, 4 the self-test took the LED |

The message carries the `seq` of the `ALERT` that started the alert, for
reason 3 too. It waits for the end of a USB suspend (reason 2), and is
dropped when the USB link becomes unconfigured, as `SELF_TEST_RESULT` is.

### ACK and ERROR

| Message | Body |
|---|---|
| `ACK` | `u8` type of the acknowledged command |
| `ERROR` | `u8` type of the rejected command, `u8` error: 1 unknown command, 2 bad length, 3 bad value, 4 busy, 5 lamp off |

## Connection

The lamp starts with nothing from the host: on, green, automatic
brightness, normal touch, the default touch cycle, an empty palette and
both corrections at 0. HID does not tell the lamp when a host closes it,
so custom colors, the cycle and the corrections stay until the lamp
restarts or the host changes them; only the touch mode falls back by
itself ([Touch modes](#touch-modes)).

After each connection, a host sends its settings again, in this order:

1. `GET_INFO`. A protocol major other than 2 ends here
   ([Version 1 lamps](#version-1-lamps)).
2. `GET_STATE`.
3. `SET_AUTO_BIAS`, then `SET_TOUCH_SENSITIVITY`.
4. `SET_COLOR` for each custom color.
5. `SET_TOUCH_CYCLE`, after the colors it names.
6. `SET_COLOR` with three zeros for the other custom slots, which clears a
   palette left by an earlier session.
7. `SET_STATE`.

## Version 1 lamps

A lamp with firmware 1.0.0 speaks version 1.1, the version before this
one. Its `INFO` has the same layout, with a protocol major of 1, but its
`STATE` has 9 bytes and its `SET_STATE` takes 5. A version 2 host reads its
`INFO`, then sends it nothing else and ignores what it sends, such as the
9-byte `STATE` it sends on its own at every change, which a version 2
decoder refuses (body too short). A version 2 lamp answers the 5-byte
`SET_STATE` of a version 1 host with `ERROR` (bad length).

## Compatibility

Within major version 2, a new minor version may only:

- add commands, messages, `INFO` capability bits, error codes or values of
  an enumeration;
- add fields at the end of a body.

To stay compatible with these changes:

- decoders accept a body **longer** than the size given here and ignore
  the extra bytes; a shorter body is an error;
- hosts ignore unsolicited messages of an unknown type, check a capability
  bit before using a command that a later minor version adds, and treat an
  unknown value of an enumeration as an error for that message only;
- lamps answer an unknown command with `ERROR` (unknown command).

## Test vectors

[`vectors/v2.json`](vectors/v2.json) lists commands and messages with their
exact bytes. The firmware unit tests (`firmware/test/test_protocol`) and the
Rust codec tests (`crates/disled-protocol`, used by the desktop app) both
check their codec against it, so the two sides cannot drift apart.

Fields of a vector:

- `bytes`: the significant start of the 63-byte payload, as hexadecimal
  pairs; the rest of the payload is zero.
- commands: `status` (`ok`, `unknown_command`, `bad_length`,
  `bad_value`), `type`, `seq` and, when relevant, `set_state`,
  `telemetry_period_ms`, `auto_bias`, `set_color`, `color_slot`,
  `touch_cycle`, `alert` or `touch_sensitivity`. `"canonical": false` marks
  bytes that a decoder must accept but an encoder never produces (non-zero
  unselected fields, extra body bytes).
- messages: `kind` and the decoded fields; an `info` gives its `protocol`
  version. `"canonical": false` marks a message only a decoder is tested
  with: a body longer than this version defines, or a message of a version
  1 lamp. `"decode_error"` marks a message that a version 2 decoder
  refuses, with the reason: `body_too_short`, for the 9-byte `STATE` of a
  version 1 lamp. The firmware tests leave these out.
