# Architecture

DISLED has three parts that share one protocol:

```
 ┌────────────── lamp (CH32V203) ──────────────┐        ┌──────── desktop app (Tauri) ────────┐
 │ touch ─┐                                    │  USB   │ Rust: USB worker thread, codec      │
 │ light ─┼─> lamp state ─> LED output (PWM)   │<──────>│ Vue: controls, telemetry, CSV       │
 │ host ──┘        └─> notifications           │  HID   │                                     │
 └─────────────────────────────────────────────┘        └─────────────────────────────────────┘
                      protocol/README.md + protocol/vectors/v2.json
```

The lamp owns its state. The touch changes it locally, the host reads and
changes it, and the lamp notifies every change. [behavior.md](behavior.md)
specifies the behavior itself.

## Firmware

### Layers

| Layer | Path | Depends on | Role |
|---|---|---|---|
| Application | `src/app/` | everything below | wiring, scheduling, host protocol, debug commands, tunables |
| Diagnostics | `src/diag/` | usb, core | logs and line console over CDC (debug builds) |
| USB | `src/usb/` | bsp, TinyUSB, core | descriptors, link state, HID queues, CDC stream |
| Drivers | `src/drivers/` | bsp, SDK | LED PWM, TouchKey, I2C, VEML7700 |
| Board support | `src/bsp/` | SDK | pin map, clocks, time base, watchdog, unique ID, faults |
| Core | `lib/disled_core/` | C library only | all decisions: state machine, touch detection, brightness, color, power cap, PWM dithering, self-test, protocol codec, scheduler, formatter, assertions |

Only `src/bsp/ch32.h` includes the vendor device header. Includes are
relative to `src/` (`#include "drivers/led_pwm.h"`), so every include shows
the layer it reaches into. The core never includes an SDK header, so it
builds natively and runs under Unity in CI.

### Data flow

```
TIM3 CC4 interrupt (1 kHz, fixed PWM phase) ─> TouchKey sample ─> queue
                                                                    │
main loop ── touch task ─> touch_detector ─> short/long press ─> lamp_fsm
          ── ALS task (100 ms) ─> VEML7700 count at one range, every 400 ms
                                     ─> lux ─> brightness
          ── output task (10 ms) ─> led_output: palette, level, fade,
                                     power cap, pulse loss ─> duties in
                                     1/16 tick ─> led_pwm
          ── usb task ─> TinyUSB, link state (unconfigured/configured/suspended)
          ── host task ─> commands, STATE notifications, telemetry
          ── console task (debug) ─> text commands, watch
          └─ watchdog feed, then the next round at once (no WFI: it hangs on this chip)

TIM3 update interrupt (20 kHz, every PWM period) ─> dithers the duties
                                     into whole ticks for the next period
```

### Concurrency

The firmware runs bare metal, in one cooperative loop driven by a
table-based scheduler (`lib/disled_core/scheduler`). Interrupt handlers
only capture data:

| Interrupt | Work |
|---|---|
| SysTick, 1 kHz | millisecond counter, arms the next TouchKey sample |
| TIM3 update, 20 kHz | dithers the three LED duties into whole ticks for the next period |
| TIM3 channel 4 | one TouchKey measurement (about 5 µs), pushed to a lock-free queue |
| USB (three vectors, one handler) | TinyUSB event capture; during suspend, first restores the full-speed clock (EXTI line 18, USB wake-up) |
| HardFault, NMI | saves `mcause`, `mepc`, `mtval` in backup registers, resets |

All interrupts share one priority, so the QingKe hardware stack never
nests.

### Safety rules

- **One path to the LED.** Only `led_output` computes duties, and it
  always applies the current cap of the USB state
  ([behavior.md](behavior.md#usb-states-and-led-power-limits)). The cap
  uses worst-case currents and scales the three channels together, which
  keeps the hue.
- **Shared pins.** The firmware never configures PA10, bonded to USB D−,
  nor PB8, bonded to BOOT0. A static assertion guards the unused-pin masks.
- **No dynamic allocation, no floating point, no 64-bit division.** The
  code uses fixed point everywhere, with static assertions on the
  intermediate ranges. The 64-bit division of libgcc alone would cost
  2 KB of flash.
- **Every wait is bounded.** I2C waits time out after 3 ms. A missing light
  sensor leaves a fixed brightness, and the firmware probes it again every
  5 s. A TouchKey conversion gives up after about 100 µs.
- **Watchdog.** The loop feeds the independent watchdog (1 s) only after
  every task ran. The longest task took 0.5 ms when measured during
  development, 1.7 ms on the suspend clock: far within that second.
- **Faults are reported.** A crash or a failed `DISLED_ASSERT` leaves its
  cause in the backup registers, then the chip resets. A failed assertion
  shows as `mcause` 0x0A55nnnn, nnnn being its source line. The next boot reports
  the fault in `INFO` and in `DIAGNOSTICS`. `DIAGNOSTICS`
  also counts boots, watchdog resets and faults since power-up, and gives
  the smallest stack margin (the boot fills the stack with a pattern) and
  error counters.
- **Crystal failure.** The firmware detects a failed crystal. It then keeps
  USB off, since the clock would be out of tolerance, and still runs the
  lamp locally.

### Power

The core never sleeps, because WFI in the main loop hangs on this chip.
While the host sleeps (USB suspend), the firmware stops the LED timer, the
touch and the light sensor, and runs the core at 1 MHz from the internal
oscillator, with the crystal and the PLL off. The USB wake-up interrupt
restores the full clock before TinyUSB handles the resume.

### Memory

32 KB flash, 10 KB RAM. `scripts/size_budget.py` prints the usage after each
build, warns above 90 % and fails above 100 %. Link-time optimization is
on.

| Build | Flash | RAM (2 KB stack included) |
|---|---|---|
| debug (HID + CDC console) | 23.0 KB | 4.8 KB |
| release (HID) | 18.0 KB | 4.1 KB |

### Configuration

Every tunable lives in `src/app/app_config.c`, with the reason for its
value. [bringup.md](bringup.md) checks them on a new board.

### LED output

The PWM runs at 20 kHz with 3600 ticks of 14 ns. Duties are in sixteenths
of a tick: at the bottom of the range a channel lasts only a few ticks, and
whole ticks would make dim fades step and the color drift. The TIM3 update
interrupt spreads the fraction over 16 periods (`pwm_dither`). Each lit
channel also gets a fixed pulse loss added, the part of every pulse that
the driver edges turn into no light. Without it, the 1-tick blue of warm
white at 10 % gives no light and the white turns yellow.

### Touch detection

`touch_detector` averages the TouchKey samples over 20 ms, one mains
period, and measures the drop below a slow baseline, in per mille. A press
must reach an absolute threshold **and** rise above the floor, the lowest
value of the last 320 ms. It ends halfway back down, or under the absolute
release threshold. A hand resting near the lid lifts the signal to the
level of a light touch, and only this rise separates the taps on top of it.
The baseline freezes while a hand is near, until it stays for the object
time, so that it cannot drift under the hand and blunt the next touch.

### Testing

- `pio test -e native` runs the core on the build machine (Unity): lamp
  rules, touch detection on synthetic signals (mains hum, drift, objects,
  finger at power-up, hand resting near the lid), brightness, color, power
  cap, output fades and pulse loss, PWM dithering, self-test, scheduler,
  formatter and the protocol codec.
- The protocol tests check the codec byte for byte against
  `protocol/vectors/v2.json`, the same file the app tests use.
- `pio check` runs cppcheck on `src/` and the core at every severity.
- [bringup.md](bringup.md) lists the checks on a real board.

## Desktop app

- **Rust backend** (`app/src-tauri`). A worker thread owns the USB
  connection through `hidapi`. It finds the lamp, reconnects after an
  unplug, writes commands and decodes messages. The results reach the web
  view as Tauri events (`device-connection`, `device-message`). The codec
  lives in its own crate, `crates/disled-protocol`, shared with the
  production tool and tested against the shared vectors.
- **Tray and windows.** The app lives in the menu bar or the notification
  area, and `tray.rs` draws its icon at run time from the lamp state. Three
  windows share one page, chosen by the window label: the popup for daily
  use, the Diagnostics window and the lamp update window. The Diagnostics
  window starts the telemetry stream at 250 ms when it opens, and the
  backend stops it when the window closes.
- **Settings.** The backend saves the settings with the store plugin and
  broadcasts them to every window (`settings-changed`). The USB worker
  applies the brightness ones (mode, fixed level, correction) as soon as
  the lamp answers `INFO`, so they hold across lamp restarts. The backend
  applies the color mode to the window frames and the language to the tray
  and the window titles (`i18n.rs`), and tells the windows the language in
  force (`locale-changed`).
- **Updates.** A thread checks the `latest.json` of the latest GitHub
  release at start-up and once a day (`updates.rs`, updater plugin). The
  popup shows the result (`update-status`) and asks the backend to install
  it. The first launch of a new version, as after an update, checks at once
  and opens the popup, so that a lamp update shows right after the app's.
- **Lamp updates.** The same thread looks for the newest signed `fw-v*`
  release (`firmware.rs`). An update runs on its own thread: download,
  signature check against the update key, `wchisp probe` every second until
  the ROM bootloader shows up, `wchisp flash`, then the `INFO` of the
  restarted lamp. wchisp is a separate program next to the app, driven
  through its log lines (`crates/disled-flash`, shared with the production
  tool). The lamp update window follows `firmware-status`.
- **Vue front end** (`app/src`, Nuxt UI). Each window runs its own Pinia
  stores (lamp, settings, telemetry, diagnostics, updates, firmware,
  errors), fed by the backend events through a typed layer (`ipc/`). The
  components only render them and send commands. ts-rs generates the types
  they exchange with the backend from the Rust types (`bindings/`), and the
  messages of both languages live in `locales/`, shared with the backend.
- USB access stays in Rust because the macOS web view has no WebHID, and
  because a connection owned by the backend keeps working while every
  window is hidden.

## Production tool

`tools/disled-factory` is a command-line program for batches of lamps
([production.md](production.md)). Board after board, it waits for the ROM
bootloader, flashes the signed release build with wchisp, checks the
`INFO` of the restarted lamp, runs `SELF_TEST` while the operator watches
the colors and touches the lid, and appends a row to a CSV log. It shares
two crates with the app: the protocol codec and the USB IDs
(`crates/disled-protocol`), and the flashing code, wchisp and the firmware
signatures (`crates/disled-flash`).
