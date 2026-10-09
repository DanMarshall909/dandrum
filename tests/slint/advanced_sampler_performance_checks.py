"""Independent native pointer/keyboard checks; run on a private headless display."""
import math
import re
import time


def performance_controls(session):
    prefix = "AdvancedSamplerPerformanceTest"
    pad = session.find(f"{prefix}::pad")
    knob = session.find(f"{prefix}::knob")
    report = session.find(f"{prefix}::event-report")
    checks = 0

    def counts():
        return tuple(int(x) for x in re.findall(r"\d+", session.properties(report)["accessibleLabel"]))

    def expect(result, reason):
        nonlocal checks
        checks += 1
        if not result:
            raise AssertionError(reason)

    session.call("click_element", elementHandle=pad)
    expect(counts()[:2] == (1, 1), "Pointer press/release must balance one pad note")
    expect(75 <= counts()[2] <= 90, "Centre pad input produces position-derived velocity")
    session.key(" ", "Press")
    session.key(" ", "Press")
    expect(counts()[:2] == (2, 1), "Repeated held Space cannot duplicate note-on")
    session.key(" ", "Release")
    expect(counts()[:2] == (2, 2), "Space release balances its held note")
    session.call("click_element", elementHandle=session.find(f"{prefix}::disable-pad-button"))
    expect(counts()[:2] == (3, 3), "Disabling a held pad releases the captured source")

    session.call("click_element", elementHandle=knob)
    session.key("\uf729")  # Home
    expect(float(session.properties(knob)["accessibleValue"]) == 0, "Home reaches parameter minimum")
    geometry = session.properties(knob)
    x = geometry["absolutePosition"]["x"] + geometry["size"]["width"] / 2
    y = geometry["absolutePosition"]["y"] + geometry["size"]["height"] / 2
    time.sleep(.51)
    before = counts()[3:]
    session.call("drag_element", elementHandle=knob, target={"x": x, "y": y - 40})
    value = float(session.properties(knob)["accessibleValue"])
    expect(math.isclose(value, .2, abs_tol=.011), "Forty-pixel drag changes one fifth of range")
    expect(counts()[3] == before[0] + 1 and counts()[4] == before[1] + 1,
           "A continuous pointer drag has one gesture begin/end")
    session.key("\n")
    for char in "0.73":
        session.key(char)
    session.key("\n")
    expect(math.isclose(float(session.properties(knob)["accessibleValue"]), .73, abs_tol=.001),
           "Native typing commits the intended actual value")
    session.screenshot("advanced-performance-input.png")
    return f"PASS: {checks} independent performance input checks"
