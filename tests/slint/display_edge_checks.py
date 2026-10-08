"""Physical input regressions for editing, audition and inspector lifecycles."""
import math
import re
from menu_checks import item, matching


def display_edges(session):
    failures, checks = [], 0
    key_map = session.find("DisplayEdges::map")
    stack = session.find("DisplayEdges::stack")

    def check(ok, reason):
        nonlocal checks
        checks += 1
        if not ok:
            failures.append(reason)

    def state():
        label = session.properties(session.find("DisplayEdges::event-report"))["accessibleLabel"]
        values = [int(value) for value in re.findall(r"\d+", label)]
        return dict(zip("edits notes releases zoom lo hi vel_lo vel_hi inspections".split(), values))

    def details_open():
        return bool(matching(session, stack, "Close source details", "Button"))

    zone = item(session, key_map, "Editable zone, keys 48–54, velocity 32–96", "Button")
    session.call("click_element", elementHandle=zone)
    session.key(" ", "Press")
    check(state()["notes"] == 1 and state()["releases"] == 0, "Holding a zone must start one audition")
    session.call("click_element", elementHandle=session.find("DisplayEdges::disable-map"))
    session.key(" ", "Release")
    check(state()["notes"] == 1 and state()["releases"] == 1, "Disabling a held map must send exactly one release")
    geometry = session.properties(zone)
    x = geometry["absolutePosition"]["x"] + geometry["size"]["width"] / 2
    y = geometry["absolutePosition"]["y"] + geometry["size"]["height"] / 2
    before = state()["edits"]
    session.call("drag_element", elementHandle=zone, target={"x": x + 80, "y": y - 20})
    check(state()["edits"] == before, "Disabled maps must reject a physical zone drag")
    session.call("click_element", elementHandle=session.find("DisplayEdges::disable-map"))
    for modifier in ["control", "meta"]:
        session.call("scroll_element", elementHandle=zone, deltaY=120, modifiers={modifier: True})
        check(100 < state()["zoom"] <= 800, f"{modifier.title()} wheel must zoom the map within its declared range")
        session.call("click_element", elementHandle=item(session, key_map, "Fit", "Button"))
        check(state()["zoom"] == 100, "Fit must restore the complete key range")
    session.call("scroll_element", elementHandle=zone, deltaY=120)
    check(state()["zoom"] == 100, "Ordinary scrolling must not change map zoom")
    for _ in range(2):
        session.call("click_element", elementHandle=item(session, key_map, "+", "Button"))
    viewport = session.find("KeyMap::viewport")
    grid = session.find("KeyMap::grid")
    origin = session.properties(viewport)["absolutePosition"]["x"]
    bar = item(session, key_map, "Horizontal scroll", "Slider")
    session.call("click_element", elementHandle=bar)
    session.key("\uf72b")
    scroll_offset = lambda: origin - session.properties(grid)["absolutePosition"]["x"]
    session.wait_until(lambda: scroll_offset() > 0)
    after_bar = scroll_offset()
    check(math.isclose(float(session.properties(bar)["accessibleValue"]), after_bar, abs_tol=.05),
          "A KeyMap scrollbar action must move the actual grid to its accepted offset")
    session.call("scroll_element", elementHandle=viewport, deltaX=120)
    session.wait_until(lambda: scroll_offset() < after_bar)
    check(scroll_offset() < after_bar, "Content wheel input must still move the map after using its scrollbar")
    check(math.isclose(float(session.properties(bar)["accessibleValue"]), scroll_offset(), abs_tol=.05),
          "The KeyMap scrollbar must track content-wheel movement after its own input changed the offset")
    session.call("click_element", elementHandle=item(session, key_map, "Fit", "Button"))

    probe = session.find("DisplayEdges::corner-probe")
    geometry = session.properties(probe)
    x = geometry["absolutePosition"]["x"] + geometry["size"]["width"] / 2
    y = geometry["absolutePosition"]["y"] + geometry["size"]["height"] / 2
    before = state()["edits"]
    session.call("drag_element", elementHandle=probe, target={"x": x + 40, "y": y - 20})
    proposal = state()
    check(proposal["edits"] == before + 1, "One corner drag must emit one zone proposal")
    check((proposal["lo"], proposal["hi"], proposal["vel_lo"], proposal["vel_hi"]) == (48, 56, 32, 106),
          "Dragging the upper-right corner must resize both note and velocity bounds")

    source = item(session, stack, "Inspect Sample source Prepared layer", "Button")
    session.call("click_element", elementHandle=source)
    check(details_open(), "Clicking a source must open its details")
    session.call("click_element", elementHandle=source)
    check(not details_open(), "Clicking the selected source again must close its details")
    if details_open():
        session.call("click_element", elementHandle=item(session, stack, "Close source details", "Button"))
    module = item(session, stack, "Inspect Filter", "Button")
    session.call("click_element", elementHandle=module)
    check(details_open(), "Clicking a module must open its details")
    session.key("\u001b")
    check(not details_open(), "Escape must close module details after a source-row click")
    if details_open():
        session.call("click_element", elementHandle=item(session, stack, "Close source details", "Button"))
    before = state()["inspections"]
    session.key("\n")
    check(details_open() and state()["inspections"] > before, "Closing details must restore keyboard focus to their source module")

    waveform = session.find("DisplayEdges::selected-waveform")
    names = matching(session, waveform, "1 Selected slice names remain readable beyond a narrow region")
    check(len(names) == 1 and names[0]["size"]["width"] > 300,
          "A selected slice name must extend beyond a narrow region without truncating at 150 pixels")
    session.call("click_element", elementHandle=session.find("DisplayEdges::selected-label-probe"))
    report = session.properties(session.find("DisplayEdges::event-report"))["accessibleLabel"]
    check(report.endswith("slice: first"), "The extended selected slice label must receive input above later slice flags")
    if details_open():
        session.call("click_element", elementHandle=item(session, stack, "Close source details", "Button"))
    binding_adapters(session, stack, check, verify_gestures=True)
    session.screenshot("display-edges-interactions.png")
    if failures:
        raise AssertionError("; ".join(failures))
    return f"PASS: {checks} independent display capability, scrolling, corner resize, inspector, slice-label and host refresh checks"


def binding_adapters(session, stack, check, prefix="DisplayEdges", verify_gestures=False):
    def bound():
        text = session.properties(session.find(f"{prefix}::binding-report"))["accessibleLabel"]
        match = re.fullmatch(r"Bound level: ([-\d.]+); send: ([-\d.]+); muted: ([01]); edits: (\d+),(\d+),(\d+)", text)
        if not match:
            raise AssertionError(f"Unexpected layer binding report: {text}")
        level, send, muted, *counts = match.groups()
        return float(level), float(send), bool(int(muted)), tuple(map(int, counts))

    def gestures():
        text = session.properties(session.find(f"{prefix}::gesture-report"))["accessibleLabel"]
        match = re.fullmatch(r"Level gestures: (\d+),(\d+); send gestures: (\d+),(\d+)", text)
        if not match:
            raise AssertionError(f"Unexpected display gesture report: {text}")
        return tuple(map(int, match.groups()))

    def type_number(handle, value):
        session.call("click_element", elementHandle=handle)
        for character in str(value):
            session.key(character)
        session.key("\n")
        session.key("\u001b")

    level = lambda: item(session, stack, "Binding layer level", "Spinbox")
    send = lambda: item(session, stack, "Room send", "Spinbox")

    def mute():
        mutes = [element for element in matching(session, stack, "Mute", "Switch")
                 if element.get("accessibleEnabled", False)]
        if len(mutes) != 1:
            raise AssertionError(f"Expected one editable layer mute, found {len(mutes)}")
        return mutes[0]["handle"]

    type_number(level(), -6)
    check(bound()[0] == -6 and bound()[3] == (1, 0, 0), "A layer level edit must reach and be acknowledged by its host model once")
    type_number(send(), -9)
    check(bound()[1] == -9 and bound()[3] == (1, 1, 0), "A send edit must reach and be acknowledged by its host model once")
    session.call("click_element", elementHandle=mute())
    check(bound()[2] and bound()[3] == (1, 1, 1), "A layer mute edit must reach and be acknowledged by its host model once")
    if verify_gestures:
        before_gestures = gestures()
        level_begins, level_ends, send_begins, send_ends = before_gestures
        check(level_begins == level_ends and level_begins > 0 and send_begins == send_ends and send_begins > 0,
              "Physical layer and send edits must forward balanced gesture callbacks")
    before = bound()[3]
    session.call("click_element", elementHandle=session.find(f"{prefix}::host-refresh"))
    check(float(session.properties(level())["accessibleValue"]) == -18,
          "A layer level control must display a later supplied host value after local editing")
    check(float(session.properties(send())["accessibleValue"]) == -24,
          "A send control must display a later supplied host value after local editing")
    check(not session.properties(mute()).get("accessibleChecked", False),
          "A mute control must display a later supplied host value after local toggling")
    check(bound() == (-18, -24, False, before), "Incoming layer and send model updates must not echo user callbacks")
    if verify_gestures:
        check(gestures() == before_gestures, "Incoming layer and send model updates must not emit gestures")


def display_bindings(session):
    failures, checks = [], 0

    def check(ok, reason):
        nonlocal checks
        checks += 1
        if not ok:
            failures.append(reason)

    binding_adapters(session, session.root, check, prefix="DisplayBindings", verify_gestures=True)
    session.screenshot("display-bindings-interactions.png")
    if failures:
        raise AssertionError("; ".join(failures))
    return f"PASS: {checks} independent persistent layer/send host binding checks"
