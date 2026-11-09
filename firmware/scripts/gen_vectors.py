# SPDX-License-Identifier: GPL-3.0-or-later
# SPDX-FileCopyrightText: 2026 The Disled contributors
# Additional permission under GNU GPL version 3 section 7: see LICENSING.md.
"""Turn protocol/vectors/v1.json and v2.json into C headers for the unit tests.

Runs before the native build. The headers land in the build directory, so
they never go stale in the source tree. The tests include the one of the
protocol version they build (protocol_vectors_v1.h or protocol_vectors_v2.h).
"""

import json
import os

Import("env")  # noqa: F821 - provided by PlatformIO

STATUS = {
    "ok": "PROTO_OK",
    "unknown_command": "PROTO_ERR_UNKNOWN_COMMAND",
    "bad_length": "PROTO_ERR_BAD_LENGTH",
    "bad_value": "PROTO_ERR_BAD_VALUE",
}
KINDS = {"state": "MSG_STATE", "info": "MSG_INFO", "telemetry": "MSG_TELEMETRY",
         "diagnostics": "MSG_DIAGNOSTICS", "self_test_result": "MSG_SELF_TEST_RESULT",
         "ack": "MSG_ACK", "error": "MSG_ERROR", "color": "MSG_COLOR",
         "touch_cycle": "MSG_TOUCH_CYCLE", "alert_done": "MSG_ALERT_DONE"}


def c_bytes(hex_text):
    values = [int(pair, 16) for pair in hex_text.split()]
    return "{ " + ", ".join("0x%02x" % v for v in values) + " }"


def c_bool(value):
    return "true" if value else "false"


def c_rgb(rgb):
    return "{ %d, %d, %d }" % tuple(rgb)


def c_cycle(cycle):
    return "{ .count = %d, .slots = { %s } }" % (cycle["count"],
                                               ", ".join(str(s) for s in cycle["slots"]))


def command_entry(vector, v2):
    set_state = vector.get("set_state")
    request = "{ 0 }"
    if set_state is not None:
        touch_mode = ""
        if v2:
            touch_mode = ", .touch_mode = (touch_mode_t)%d" % set_state.get("touch_mode", 0)
        request = "{ .fields = %d, .on = %s, .color = (lamp_color_t)%d, " \
                  ".brightness_mode = (brightness_mode_t)%d, .manual_level = %d%s }" % (
                      set_state["fields"], c_bool(set_state.get("on", False)),
                      set_state.get("color", 0), set_state.get("brightness_mode", 0),
                      set_state.get("manual_level", 0), touch_mode)
    period = vector.get("telemetry_period_ms")
    bias = vector.get("auto_bias")
    entry = ("    { .name = \"%s\", .payload = %s, .status = %s, .type = 0x%02x, .seq = %d,\n"
             "      .has_set_state = %s, .set_state = %s,\n"
             "      .has_period = %s, .telemetry_period_ms = %d,\n"
             "      .has_auto_bias = %s, .auto_bias = %d") % (
                 vector["name"], c_bytes(vector["bytes"]), STATUS[vector["status"]], vector["type"],
                 vector["seq"], c_bool(set_state is not None), request, c_bool(period is not None),
                 period or 0, c_bool(bias is not None), bias or 0)
    if v2:
        set_color = vector.get("set_color")
        slot = vector.get("color_slot")
        cycle = vector.get("touch_cycle")
        alert = vector.get("alert")
        sensitivity = vector.get("touch_sensitivity")
        entry += (",\n      .has_set_color = %s, .set_color_slot = %d, .set_color = %s,\n"
                  "      .has_color_slot = %s, .color_slot = %d,\n"
                  "      .has_touch_cycle = %s, .touch_cycle = %s,\n"
                  "      .has_alert = %s, .alert_count = %d, .alert_period_ms = %d,\n"
                  "      .has_touch_sensitivity = %s, .touch_sensitivity = %d") % (
                      c_bool(set_color is not None), set_color["slot"] if set_color else 0,
                      c_rgb(set_color["rgb"]) if set_color else "{ 0, 0, 0 }",
                      c_bool(slot is not None), slot or 0, c_bool(cycle is not None),
                      c_cycle(cycle) if cycle else "{ 0 }", c_bool(alert is not None),
                      alert["count"] if alert else 0, alert["period_ms"] if alert else 0,
                      c_bool(sensitivity is not None), sensitivity or 0)
    return entry + " },\n"


def message_entry(vector, v2):
    kind = vector["kind"]
    fields = []
    if kind == "state":
        s = vector["state"]
        touch_mode = ", .touch_mode = (touch_mode_t)%d" % s["touch_mode"] if v2 else ""
        rgb = ", .rgb = %s" % c_rgb(s["rgb"]) if v2 else ""
        fields.append(".state = { .state_seq = %d, .lamp = { .on = %s, .color = (lamp_color_t)%d, "
                      ".brightness_mode = (brightness_mode_t)%d, .manual_level = %d%s }, "
                      ".applied_percent = %d, .link = (usb_link_t)%d, .source = (change_source_t)%d%s }"
                      % (s["state_seq"], c_bool(s["on"]), s["color"], s["brightness_mode"],
                         s["manual_level"], touch_mode, s["applied_percent"], s["link"],
                         s["source"], rgb))
    elif kind == "info":
        i = vector["info"]
        uid = ", ".join("0x%s" % i["uid"][k:k + 2] for k in range(0, len(i["uid"]), 2))
        fields.append(".info = { .fw_major = %d, .fw_minor = %d, .fw_patch = %d, "
                      ".board_revision = %d, .uid = { %s }, .capabilities = %du, "
                      ".reset_cause = (reset_cause_t)%d, .flags = %d, .fault_mcause = 0x%08xu, "
                      ".fault_mepc = 0x%08xu, .fw_version = \"%s\" }"
                      % (i["fw_major"], i["fw_minor"], i["fw_patch"], i["board_revision"], uid,
                         i["capabilities"], i["reset_cause"], i["flags"], i["fault_mcause"],
                         i["fault_mepc"], i["fw_version"]))
    elif kind == "telemetry":
        t = vector["telemetry"]
        fields.append(".telemetry = { .uptime_ms = %du, .touch_raw = %d, .touch_value = %d, "
                      ".touch_baseline = %d, .touch_delta_permille = %d, .touch_state = %d, "
                      ".lux_centi = %du, .als_raw = %d, .duty = { %d, %d, %d }, "
                      ".led_current_ma_x10 = %d, .cap_ma = %d, .link = (usb_link_t)%d, "
                      ".level_permille = %d }"
                      % (t["uptime_ms"], t["touch_raw"], t["touch_value"], t["touch_baseline"],
                         t["touch_delta_permille"], t["touch_state"], t["lux_centi"], t["als_raw"],
                         t["duty"][0], t["duty"][1], t["duty"][2], t["led_current_ma_x10"],
                         t["cap_ma"], t["link"], t["level_permille"]))
    elif kind == "diagnostics":
        d = vector["diagnostics"]
        sensitivity = ", .touch_sensitivity = %d" % d["touch_sensitivity"] if v2 else ""
        fields.append(".diagnostics = { .uptime_ms = %du, .boots = %d, .watchdog_resets = %d, "
                      ".faults = %d, .fault_mcause = 0x%08xu, .fault_mepc = 0x%08xu, "
                      ".fault_mtval = 0x%08xu, .stack_free_min = %d, .i2c_errors = %d, "
                      ".light_sensor_probes = %d, .touch_samples_lost = %d, "
                      ".touch_recalibrations = %d, .usb_suspends = %d, .auto_bias = %d, "
                      ".flags = %d%s }"
                      % (d["uptime_ms"], d["boots"], d["watchdog_resets"], d["faults"],
                         d["fault_mcause"], d["fault_mepc"], d["fault_mtval"], d["stack_free_min"],
                         d["i2c_errors"], d["light_sensor_probes"], d["touch_samples_lost"],
                         d["touch_recalibrations"], d["usb_suspends"], d["auto_bias"],
                         d["flags"], sensitivity))
    elif kind == "self_test_result":
        r = vector["self_test_result"]
        fields.append(".self_test_result = { .results = %d, .lux_centi = %du, "
                      ".touch_baseline = %d, .touch_peak_permille = %d }"
                      % (r["flags"], r["lux_centi"], r["touch_baseline"],
                         r["touch_peak_permille"]))
    elif kind == "ack":
        fields.append(".command = 0x%02x" % vector["command"])
    elif kind == "error":
        fields.append(".command = 0x%02x, .error = (proto_status_t)%d" % (vector["command"],
                                                                         vector["error"]))
    elif kind == "color":
        c = vector["color"]
        fields.append(".color_slot = %d, .color = %s" % (c["slot"], c_rgb(c["rgb"])))
    elif kind == "touch_cycle":
        fields.append(".touch_cycle = %s" % c_cycle(vector["touch_cycle"]))
    elif kind == "alert_done":
        fields.append(".reason = %d" % vector["reason"])
    fields.append(".canonical = %s" % c_bool(vector.get("canonical", True)))
    return "    { .name = \"%s\", .kind = %s, .seq = %d, .payload = %s,\n      %s },\n" % (
        vector["name"], KINDS[kind], vector["seq"], c_bytes(vector["bytes"]), ", ".join(fields))


def header(doc, v2):
    command_fields = ("    bool has_auto_bias;\n    int8_t auto_bias;\n")
    message_fields = ("    proto_status_t error;\n")
    kinds = ("    MSG_STATE,\n    MSG_INFO,\n    MSG_TELEMETRY,\n    MSG_DIAGNOSTICS,\n"
             "    MSG_SELF_TEST_RESULT,\n    MSG_ACK,\n    MSG_ERROR")
    if v2:
        command_fields += ("    bool has_set_color;\n    uint8_t set_color_slot;\n"
                           "    rgb16_t set_color;\n    bool has_color_slot;\n"
                           "    uint8_t color_slot;\n    bool has_touch_cycle;\n"
                           "    touch_cycle_t touch_cycle;\n    bool has_alert;\n"
                           "    uint8_t alert_count;\n    uint16_t alert_period_ms;\n"
                           "    bool has_touch_sensitivity;\n    int8_t touch_sensitivity;\n")
        message_fields += ("    uint8_t color_slot;\n    rgb16_t color;\n"
                           "    touch_cycle_t touch_cycle;\n    uint8_t reason;\n")
        kinds += ",\n    MSG_COLOR,\n    MSG_TOUCH_CYCLE,\n    MSG_ALERT_DONE"
    return [
        "/* Generated from protocol/vectors/v%d.json by scripts/gen_vectors.py. Do not edit. */\n"
        % (2 if v2 else 1),
        "#include <stdbool.h>\n#include <stdint.h>\n\n#include \"protocol.h\"\n\n",
        "#define VECTORS_PROTOCOL_MAJOR %du\n#define VECTORS_PROTOCOL_MINOR %du\n\n"
        % tuple(doc["protocol_version"]),
        "typedef struct {\n    const char *name;\n    uint8_t payload[PROTO_REPORT_SIZE];\n"
        "    proto_status_t status;\n    uint8_t type;\n    uint8_t seq;\n    bool has_set_state;\n"
        "    lamp_request_t set_state;\n    bool has_period;\n    uint16_t telemetry_period_ms;\n"
        + command_fields + "} command_vector_t;\n\n",
        "typedef enum {\n" + kinds + "\n} message_kind_t;\n\n",
        "typedef struct {\n    const char *name;\n    message_kind_t kind;\n    uint8_t seq;\n"
        "    uint8_t payload[PROTO_REPORT_SIZE];\n    proto_state_t state;\n    proto_info_t info;\n"
        "    proto_telemetry_t telemetry;\n    proto_diagnostics_t diagnostics;\n"
        "    proto_self_test_result_t self_test_result;\n    uint8_t command;\n"
        + message_fields + "    bool canonical; /* false: decoder-only vector */\n"
        "} message_vector_t;\n\n",
        "static const command_vector_t command_vectors[] = {\n",
    ]


def generate(source, target, v2):
    with open(source, encoding="utf-8") as handle:
        doc = json.load(handle)
    lines = header(doc, v2)
    lines += [command_entry(v, v2) for v in doc["commands"]]
    lines += ["};\n\nstatic const message_vector_t message_vectors[] = {\n"]
    lines += [message_entry(v, v2) for v in doc["messages"]]
    lines += ["};\n"]
    text = "".join(lines)
    # Every test suite runs this script, and on Windows rewriting the header
    # can fail while another process still holds it open.
    try:
        with open(target, encoding="utf-8", newline="\n") as handle:
            if handle.read() == text:
                return
    except FileNotFoundError:
        pass
    os.makedirs(os.path.dirname(target), exist_ok=True)
    with open(target, "w", encoding="utf-8", newline="\n") as handle:
        handle.write(text)


project_dir = env.subst("$PROJECT_DIR")  # noqa: F821
generated_dir = os.path.join(env.subst("$BUILD_DIR"), "generated")  # noqa: F821
for version in (1, 2):
    generate(os.path.join(project_dir, "..", "protocol", "vectors", "v%d.json" % version),
             os.path.join(generated_dir, "protocol_vectors_v%d.h" % version), version == 2)
env.Append(CPPPATH=[generated_dir])  # noqa: F821
