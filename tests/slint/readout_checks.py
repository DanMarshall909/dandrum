"""Actual input and rendered geometry of layer parameters and fixed readouts."""
import math
import re
from menu_checks import elements, item, matching


def layer_details(session):
    checks = 0

    def check(ok, reason):
        nonlocal checks
        checks += 1
        if not ok:
            raise AssertionError(reason)

    def state():
        label = session.properties(session.find("LayerDetailsTests::event-report"))["accessibleLabel"]
        match = re.fullmatch(r"Numbers: (\d+); choices: (\d+); value: ([-\d.]+); option: ([^;]*); close: (\d+); begins: (\d+); ends: (\d+)", label)
        if not match:
            raise AssertionError(f"Unexpected layer event report: {label}")
        numbers, choices, value, option, closes, begins, ends = match.groups()
        return {"numbers": int(numbers), "choices": int(choices), "value": float(value), "option": option,
                "closes": int(closes), "begins": int(begins), "ends": int(ends)}

    session.key("\u001b")
    check(state()["closes"] == 1, "Opening layer details must focus a scope that responds to Escape")
    numeric = session.find("LayerDetailsTests::numeric")
    gain = item(session, numeric, "GAIN", "Slider")
    session.call("click_element", elementHandle=gain)
    session.key("\n")
    for character in "-12":
        session.key(character)
    session.key("\n")
    observed = state()
    check(float(session.properties(gain)["accessibleValue"]) == -12
          and observed["numbers"] == 1 and observed["value"] == -12,
          "Typing a layer gain must commit one actual negative dB value")
    check(observed["begins"] == observed["ends"] and observed["begins"] >= 1,
          "Physical layer gain edits must pair their gesture callbacks")
    session.key("\u001b")
    options = session.find("LayerDetailsTests::options")
    session.call("click_element", elementHandle=item(session, options, "HP", "Tab"))
    check(state()["choices"] == 1 and state()["option"] == "high-pass", "A layer enum must emit the supplied option ID")
    session.call("click_element", elementHandle=item(session, options, "BP", "Tab"))
    check(state()["choices"] == 1, "A disabled enum option must emit no selection")
    before_refresh = state()
    session.call("click_element", elementHandle=session.find("LayerDetailsTests::host-refresh-button"))
    check(float(session.properties(gain)["accessibleValue"]) == -9
          and session.properties(item(session, options, "LP", "Tab")).get("accessibleValue") == "selected",
          "A host model refresh must replace both a typed number and a locally selected enum")
    check(state() == before_refresh, "A host model refresh must not echo value, option or gesture callbacks")
    session.call("click_element", elementHandle=item(session, options, "HP", "Tab"))
    session.key("\uf703")
    session.wait_until(lambda: state()["option"] == "low-pass")
    check(state()["choices"] == 3, "Keyboard choice navigation must skip the disabled option and wrap once")
    prepared = item(session, session.find("LayerDetailsTests::prepared"), "PITCH", "Slider")
    before = session.properties(prepared)["accessibleValue"]
    session.call("set_element_value", elementHandle=prepared, value="1.5")
    check(session.properties(prepared)["accessibleValue"] == before and state()["numbers"] == 1,
          "Prepared source parameters must reject native accessibility edits")
    details = session.find("LayerDetailsTests::details")
    empty = session.find("LayerDetailsTests::empty")
    supplied = [element for element in elements(session, details)
                if any(entry.get("typeName") == "WaveformPanel" for entry in element.get("typeNamesAndIds", []))]
    absent = [element for element in elements(session, empty)
              if any(entry.get("typeName") == "WaveformPanel" for entry in element.get("typeNamesAndIds", []))]
    check(len(supplied) == 1 and not absent, "Only supplied source data may create a waveform view")
    check(bool(matching(session, details, "Supplied source")), "The source waveform must retain its supplied label")
    selected = item(session, details, "ONE SHOT", "Tab")
    session.call("click_element", elementHandle=item(session, details, "GATE", "Tab"))
    check(session.properties(selected).get("accessibleValue") == "selected", "A prepared source enum must retain its accepted choice")
    before = state()["closes"]
    session.call("click_element", elementHandle=item(session, details, "Close source details", "Button"))
    check(state()["closes"] == before + 1, "The source detail close button must emit one close event")
    session.screenshot("layer-details-interactions.png")
    return f"PASS: {checks} independent layer detail typing, gestures, choice, capability, waveform and focus checks"


def readouts(session):
    checks = 0
    for name, label, value, unit, alignment in [("level", "LEVEL", "−3.0", "dB", "left"),
                                                ("host", "HOST", "1.122", "×", "center"),
                                                ("large", "LARGE", "48", "kHz", "right")]:
        handle = session.find(f"ReadoutTests::{name}")
        parent = session.properties(handle)
        caption = session.properties(item(session, handle, label))
        number = session.properties(item(session, handle, value))
        suffix = session.properties(item(session, handle, unit))
        caption_bottom = caption["absolutePosition"]["y"] + caption["size"]["height"]
        if caption_bottom > number["absolutePosition"]["y"]:
            raise AssertionError(f"{name} caption overlaps its value")
        checks += 1
        left = number["absolutePosition"]["x"]
        right = suffix["absolutePosition"]["x"] + suffix["size"]["width"]
        parent_left = parent["absolutePosition"]["x"]
        parent_right = parent_left + parent["size"]["width"]
        actual, expected = ((left, parent_left) if alignment == "left" else (right, parent_right)
                            if alignment == "right" else ((left + right) / 2, (parent_left + parent_right) / 2))
        if not math.isclose(actual, expected, abs_tol=.05):
            raise AssertionError(f"{name} readout must align {alignment}: expected {expected}, got {actual}")
        checks += 1
        if number["absolutePosition"]["x"] + number["size"]["width"] > suffix["absolutePosition"]["x"]:
            raise AssertionError(f"{name} value overlaps its unit")
        checks += 1
    session.screenshot("readout-geometry.png")
    return f"PASS: {checks} independent caption, value, unit and alignment geometry checks"
