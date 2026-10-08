"""Independent pointer, keyboard and accessibility checks of the real controls."""
import math
import re
from menu_checks import control_menus, menus, elements
from display_edge_checks import display_edges, display_bindings
from scroll_geometry_checks import scroll_geometry
from readout_checks import layer_details, readouts

UP, DOWN, HOME, END, SHIFT = "\uf700", "\uf701", "\uf729", "\uf72b", "\u0010"


def value(session, handle):
    return float(session.properties(handle)["accessibleValue"])


def equal(actual, expected, reason):
    if not math.isclose(actual, expected, abs_tol=.011):
        raise AssertionError(f"{reason}: expected {expected}, got {actual}")


def labelled(session, label, role):
    matches = [element["handle"] for element in session.tree()["elements"]
               if element.get("accessibleRole") == role and element.get("accessibleLabel") == label]
    if len(matches) != 1:
        raise AssertionError(f"Expected one {role} labelled {label!r}, found {len(matches)}")
    return matches[0]


def counters(session, prefix):
    text = session.properties(session.find(f"{prefix}::event-report"))["accessibleLabel"]
    return tuple(int(value) for value in re.findall(r"\d+", text))


def type_value(session, text, commit=True):
    session.key("\n")
    for character in text:
        session.key(character)
    session.key("\n" if commit else "\u001b")


def controls(session):
    knob = session.find("ControlsTest::knob")
    slider = session.find("ControlsTest::slider")
    toggle = session.find("ControlsTest::toggle")
    field = session.find("ControlsTest::field")
    equal(value(session, knob), -3, "Knob begins at the supplied dB value")
    session.call("click_element", elementHandle=knob)
    session.key("\u001b")
    session.key(HOME)
    equal(value(session, knob), -60, "Home reaches the declared lower bound")
    session.key(UP)
    equal(value(session, knob), -56.7, "Arrow key moves five percent of the actual range")
    session.key(END)
    equal(value(session, knob), 6, "End reaches the declared upper bound")
    session.key(UP)
    equal(value(session, knob), 6, "Keyboard values are clamped")
    session.call("click_element", elementHandle=knob, action="DoubleClick")
    equal(value(session, knob), -3, "Double click resets the actual default")
    session.call("scroll_element", elementHandle=knob, deltaY=-20)
    equal(value(session, knob), -4.3, "Wheel changes the actual range and snaps to its declared step")
    session.key(HOME)
    before = value(session, knob)
    geometry = session.properties(knob)
    x = geometry["absolutePosition"].get("x", 0) + geometry["size"]["width"] / 2
    y = geometry["absolutePosition"].get("y", 0) + geometry["size"]["height"] / 2
    # Independent gestures are separated from the previous click's double-click interval.
    import time
    time.sleep(.51)
    session.call("drag_element", elementHandle=knob, target={"x": x, "y": y - 40})
    equal(value(session, knob), before + 13.2, "Forty-pixel drag spans twenty percent of a 66 dB range")
    type_value(session, "-12")
    equal(value(session, knob), -12, "Typing in the floating editor commits the actual dB value")
    type_value(session, "bad")
    equal(value(session, knob), -12, "Invalid typed text cannot change the value")
    session.key("\u001b")
    type_value(session, "2", commit=False)
    equal(value(session, knob), -12, "Escape cancels a typed edit without changing its base value")
    type_value(session, "200")
    equal(value(session, knob), 6, "Typed values clamp to the declared actual range")
    session.call("click_element", elementHandle=slider)
    session.key("\u001b")
    session.key(HOME)
    session.key(UP)
    equal(value(session, slider), 5, "Slider consumes keyboard input after receiving pointer focus")
    session.key(SHIFT, "Press")
    session.key(UP)
    session.key(SHIFT, "Release")
    equal(value(session, slider), 6, "Shift arrow selects the one-percent precision step")
    initial = session.properties(toggle).get("accessibleChecked", False)
    session.call("click_element", elementHandle=toggle)
    if session.properties(toggle).get("accessibleChecked", False) == initial:
        raise AssertionError("Pointer click did not change the toggle")
    session.key(" ")
    if session.properties(toggle).get("accessibleChecked", False) != initial:
        raise AssertionError("Space did not toggle the focused control back")
    session.call("click_element", elementHandle=field)
    session.key("\u001b")
    session.key(HOME)
    session.key(UP)
    equal(value(session, field), 2, "Numeric field arrows use the declared integer step")
    type_value(session, "7")
    equal(value(session, field), 7, "Numeric field typed entry commits an integer")
    prepared = next(element["handle"] for element in session.tree()["elements"]
                    if element.get("accessibleRole") == "Slider" and element.get("accessibleLabel") == "Prepared")
    previous = value(session, prepared)
    session.call("set_element_value", elementHandle=prepared, value="0.9")
    equal(value(session, prepared), previous, "Read-only controls reject accessibility edits")
    menu_checks = control_menus(session, knob, field, slider)
    session.screenshot("controls-interactions.png")
    return f"PASS: {19 + menu_checks} independent pointer, keyboard, typing, precision, reset, drag and menu checks"


def layout(session):
    scroll_checks = scroll_geometry(session)
    sample = labelled(session, "SAMPLE", "Tab")
    sample_geometry = session.properties(sample)
    icons = [element for element in elements(session, sample)
             if any(entry.get("typeName") == "Icon" for entry in element.get("typeNamesAndIds", []))]
    if len(icons) != 1:
        raise AssertionError("An icon-bearing navigation item must render its supplied icon")
    icon = icons[0]
    status = session.find("LayoutTests::status")
    status_geometry = session.properties(status)
    locate = labelled(session, "Locate", "Button")
    locate_geometry = session.properties(locate)
    defects = []
    if not math.isclose(icon["absolutePosition"]["y"] + icon["size"]["height"] / 2,
                        sample_geometry["absolutePosition"]["y"] + sample_geometry["size"]["height"] / 2,
                        abs_tol=.011):
        defects.append("Navigation icons must align with the vertical center of their tab")
    if locate_geometry["size"]["width"] > 96:
        defects.append("A short status action must remain compact instead of consuming the text column")
    if not math.isclose(locate_geometry["absolutePosition"]["y"] + locate_geometry["size"]["height"] / 2,
                        status_geometry["absolutePosition"]["y"] + status_geometry["size"]["height"] / 2,
                        abs_tol=.011):
        defects.append("Status actions must be vertically centered in their banner")
    if defects:
        session.screenshot("layout-geometry-failure.png")
        raise AssertionError("; ".join(defects))
    session.call("click_element", elementHandle=locate)
    session.key(" ")
    if session.properties(session.find("LayoutTests::status-report"))["accessibleLabel"] != "Status actions: 2":
        raise AssertionError("Status actions must emit once for each pointer and keyboard activation")
    rollout = session.find("LayoutTests::rollout")
    rollout_header = session.find("Rollout::header")
    rollout_geometry = session.properties(rollout)
    header_geometry = session.properties(rollout_header)
    equal(header_geometry["absolutePosition"]["y"], rollout_geometry["absolutePosition"]["y"],
          "Rollout header begins at the top of the rollout")
    title_geometry = session.properties(labelled(session, "DETAILS", "Text"))
    body_geometry = session.properties(labelled(session, "Expandable details", "Text"))
    if title_geometry["absolutePosition"]["y"] + title_geometry["size"]["height"] > body_geometry["absolutePosition"]["y"]:
        raise AssertionError("Expanded rollout title must not overlap its body")
    session.call("click_element", elementHandle=labelled(session, "DETAILS", "Button"))
    equal(session.properties(rollout)["size"]["height"], 22, "Pointer click collapses the rollout to its declared header")
    session.key("\uf703")
    equal(session.properties(rollout)["size"]["height"], rollout_geometry["size"]["height"], "Right arrow expands the focused rollout")
    session.key("\uf702")
    equal(session.properties(rollout)["size"]["height"], 22, "Left arrow collapses the focused rollout")
    session.key(" ")
    equal(session.properties(rollout)["size"]["height"], rollout_geometry["size"]["height"], "Space expands the focused rollout")
    panel = session.find("LayoutTests::panel")
    header = labelled(session, "PANEL", "Button")
    refresh = labelled(session, "Refresh", "Button")
    blocked = labelled(session, "Unavailable", "Button")
    height = session.properties(panel)["size"]["height"]
    session.call("click_element", elementHandle=refresh)
    equal(session.properties(panel)["size"]["height"], height, "Header action does not collapse the panel")
    if counters(session, "LayoutTests") != (1, 0, 0):
        raise AssertionError("Refresh must emit exactly one action without a collapse event")
    session.call("click_element", elementHandle=blocked)
    if counters(session, "LayoutTests") != (1, 0, 0):
        raise AssertionError("Disabled header action must emit nothing")
    session.call("click_element", elementHandle=header)
    equal(session.properties(panel)["size"]["height"], 28, "Header click collapses to the header height")
    session.key(" ")
    equal(session.properties(panel)["size"]["height"], height, "Space expands the focused panel")
    if counters(session, "LayoutTests") != (1, 0, 2):
        raise AssertionError("Pointer and keyboard collapse gestures each emit exactly one event")
    sample = labelled(session, "SAMPLE", "Tab")
    mapping = labelled(session, "MAPPING", "Tab")
    unavailable = labelled(session, "UNAVAILABLE", "Tab")
    session.call("click_element", elementHandle=unavailable)
    if session.properties(sample).get("accessibleValue") != "selected":
        raise AssertionError("Pointer activation of a disabled tab changed selection")
    session.call("click_element", elementHandle=sample)
    session.key("\uf703")
    session.wait_until(lambda: session.properties(mapping).get("accessibleValue") == "selected")
    session.key("\uf703")
    session.wait_until(lambda: session.properties(sample).get("accessibleValue") == "selected")
    session.call("click_element", elementHandle=labelled(session, "SPECTRUM", "Tab"))
    if session.properties(labelled(session, "SPECTRUM", "Tab")).get("accessibleValue") != "selected":
        raise AssertionError("Segment pointer activation did not select its page")
    session.call("click_element", elementHandle=session.find("LayoutTests::row"))
    session.key("\n")
    if counters(session, "LayoutTests") != (1, 2, 2):
        raise AssertionError("List row must activate once for each pointer and keyboard gesture")
    bar = session.find("LayoutTests::bar")
    session.call("click_element", elementHandle=bar)
    session.key(HOME)
    equal(value(session, bar), 0, "Scrollbar Home returns to the start")
    session.call("scroll_element", elementHandle=bar, deltaY=-90)
    equal(value(session, bar), 90, "Wheel scrolls ninety logical pixels")
    session.key(END)
    equal(value(session, bar), 400, "Scrollbar End reaches the viewport-adjusted bound")
    session.call("scroll_element", elementHandle=bar, deltaY=-90)
    equal(value(session, bar), 400, "Scrollbar wheel is clamped at the end")
    session.screenshot("layout-interactions.png")
    return f"PASS: {25 + scroll_checks} independent status action, icon geometry, header, rollout, capability, collapse, tab, segment, row and scrollbar checks"


def displays(session):
    pad = session.find("Displays::pad")
    missing = session.find("Displays::missing-pad")
    unmapped = session.find("Displays::unmapped-pad")
    session.call("click_element", elementHandle=pad)
    if counters(session, "Displays") != (1, 1, 64, 0):
        raise AssertionError("Center pad click must emit a matched note at velocity 64")
    session.key(" ", "Press")
    if counters(session, "Displays") != (2, 1, 102, 0):
        raise AssertionError("Holding Space must start exactly one audition at velocity 102")
    session.key(" ", "Press")
    if counters(session, "Displays") != (2, 1, 102, 0):
        raise AssertionError("Key repeat must not retrigger a held pad")
    session.key(" ", "Release")
    if counters(session, "Displays") != (2, 2, 102, 0):
        raise AssertionError("Releasing Space must end the held pad")
    session.call("click_element", elementHandle=missing)
    session.call("click_element", elementHandle=unmapped)
    session.key(" ")
    if counters(session, "Displays") != (2, 2, 102, 0):
        raise AssertionError("Missing and unmapped pads must remain silent for pointer and key input")
    zone = labelled(session, "Snare soft, keys 38–38, velocity 1–63", "Button")
    session.call("click_element", elementHandle=zone)
    if counters(session, "Displays") != (2, 2, 102, 0):
        raise AssertionError("Selecting a key-map zone must not audition it")
    session.key("\n", "Press")
    if counters(session, "Displays") != (3, 2, 32, 0):
        raise AssertionError("Zone audition must use the selected layer's velocity midpoint")
    session.key("\n", "Release")
    if counters(session, "Displays") != (3, 3, 32, 0):
        raise AssertionError("Releasing Enter must end the zone audition")
    geometry = session.properties(zone)
    x = geometry["absolutePosition"]["x"] + geometry["size"]["width"] / 2
    y = geometry["absolutePosition"]["y"] + geometry["size"]["height"] / 2
    session.call("drag_element", elementHandle=zone, target={"x": x + 80, "y": y - 30})
    if counters(session, "Displays") != (3, 3, 32, 0):
        raise AssertionError("Dragging a prepared zone must not mutate its mapping")
    labels = {element.get("accessibleLabel") for element in session.tree()["elements"]}
    if not {"Left: -6 dBFS", "Right: 2 dBFS", "Rear: Unavailable"}.issubset(labels):
        raise AssertionError("Meters must preserve signed channel values and distinguish unavailable measurements")
    session.screenshot("displays-interactions.png")
    return "PASS: 10 independent pad, note-lifecycle, key-map, capability and measurement checks"


def waveform(session):
    for name, left, width in [("positive", 210, 180), ("negative", 120, 90)]:
        component = session.find(f"WaveformTests::{name}")
        parent = session.properties(component)
        rails = [element for element in elements(session, component)
                 if any(entry.get("id") == "StartModulationRail::rail"
                        for entry in element.get("typeNamesAndIds", []))]
        if len(rails) != 1:
            raise AssertionError(f"Expected one rendered {name} source rail, found {len(rails)}")
        rail = rails[0]
        equal(rail["absolutePosition"].get("x", 0) - parent["absolutePosition"].get("x", 0), left,
              f"{name.title()} rail begins in the independent source coordinate")
        equal(rail["size"]["width"], width, f"{name.title()} rail spans the signed and clamped source range")
    session.screenshot("waveform-geometry.png")
    return "PASS: 4 independent rendered positive and negative source-rail geometry checks"


LIVE_CHECKS = {"Controls": controls, "Layout": layout, "Displays": displays,
               "Menus": menus, "Waveform": waveform, "DisplayEdges": display_edges,
               "LayerDetails": layer_details, "Readouts": readouts, "DisplayBindings": display_bindings}
