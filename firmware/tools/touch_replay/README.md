# touch_replay

Replays a touch recording through `touch_detector` on the build machine, to
compare detector settings before flashing them.

## Recording

On the debug console, run `watch 20`, which prints one line per 20 ms window
as the detector sees it, between the touch events of the log for
comparison. Save the console output to a file.

The replay feeds each window value to the detector 20 times, as 1 kHz
samples. The console may skip a window now and then, so the timing is close
enough to compare settings, not to time a long press to the millisecond.

## Build and run

Any C11 compiler for the build machine:

```sh
gcc -std=c11 -O2 -I../../lib/disled_core/include \
    touch_replay.c ../../lib/disled_core/src/touch_detector.c -o touch_replay
./touch_replay recording.log
./touch_replay recording.log rise=15 debounce=60
```

Settings: `press`, `release`, `rise`, `proximity` (per mille), `debounce`,
`rearm`, `long`, `object` (ms). The defaults copy `app_touch_config` from
`src/app/app_config.c`, so check that they still match.

`sensitivity` (percent, −50 to 50) applies the correction of the desktop
app's touch sensitivity setting to those thresholds, as the lamp does. The
first line of the output gives the thresholds in force. To compare every
step of the setting on one recording:

```sh
for s in -50 -40 -30 -20 -10 0 10 20 30 40 50; do
    ./touch_replay recording.log sensitivity=$s | sed -n '1p;$p'
done
```
