"""Real input checks for menus and passive, delayed tooltips."""
import math
import time

MENU, DOWN, ESCAPE = "\uf735", "\uf701", "\u001b"


def elements(session, parent):
    return session.content(session.call("get_element_tree", elementHandle=parent,
                                        maxElements=1000))["elements"]


def matching(session, parent, label, role="Text"):
    return [element for element in elements(session, parent)
            if element.get("accessibleLabel") == label and element.get("accessibleRole") == role]


def item(session, parent, label, role="Text"):
    matches = matching(session, parent, label, role)
    if len(matches) != 1:
        raise AssertionError(f"Expected one open {role} menu item {label!r}, found {len(matches)}")
    return matches[0]["handle"]


def control_menus(session, knob, field, slider):
    for handle, expected in [(knob, -3), (field, 1), (slider, 50)]:
        session.call("click_element", elementHandle=handle, button="Right")
        reset = item(session, handle, "Reset to default")
        session.call("click_element", elementHandle=reset)
        actual = float(session.properties(handle)["accessibleValue"])
        if not math.isclose(actual, expected, abs_tol=.011):
            raise AssertionError(f"Right-click menu reset: expected {expected}, got {actual}")
    session.call("click_element", elementHandle=knob)
    session.key(ESCAPE)
    session.key(MENU)
    session.call("click_element", elementHandle=item(session, knob, "Type value…"))
    for character in "-9":
        session.key(character)
    session.key("\n")
    if float(session.properties(knob)["accessibleValue"]) != -9:
        raise AssertionError("Menu key / Type value action must focus and commit the editor")
    return 4


def menus(session):
    button = session.find("MenuTests::menu-button")
    surface = session.find("MenuTests::surface")
    context = session.find("MenuTests::context")
    target = session.find("MenuTests::context-button")
    report = session.find("MenuTests::event-report")
    checks = 0

    def check(ok, reason):
        nonlocal checks
        checks += 1
        if not ok:
            raise AssertionError(reason)

    def state(actions, last, clicks):
        actual = session.properties(report)["accessibleLabel"]
        expected = f"Actions: {actions}; last: {last}; target clicks: {clicks}"
        check(actual == expected, f"Expected {expected!r}, got {actual!r}")

    session.call("click_element", elementHandle=button)
    check(bool(matching(session, button, "Aux 3–4", "Button")), "Pointer click must open the dropdown")
    session.key(DOWN)
    time.sleep(.1)
    session.key("\n")
    state(1, "aux", 0)
    check(not matching(session, button, "Aux 3–4", "Button"), "Selecting a dropdown action must close it")
    session.call("click_element", elementHandle=button)
    session.call("click_element", elementHandle=item(session, button, "Unavailable", "Button"))
    state(1, "aux", 0)
    session.key(ESCAPE)
    check(not matching(session, button, "Aux 3–4", "Button"), "Escape must dismiss the dropdown")
    session.call("click_element", elementHandle=item(session, surface, "Copy value", "Button"))
    state(2, "copy", 0)
    session.call("click_element", elementHandle=item(session, surface, "Unavailable", "Button"))
    state(2, "copy", 0)

    session.call("click_element", elementHandle=target)
    state(2, "copy", 1)
    session.call("click_element", elementHandle=target, button="Right")
    reset = item(session, context, "Reset to default")
    session.screenshot("menus-context.png")
    session.call("click_element", elementHandle=reset)
    state(3, "reset", 1)
    session.call("click_element", elementHandle=target)
    session.key(MENU)
    check(bool(matching(session, context, "Reset to default")), "Menu key must open a focused child's context menu")
    session.key(DOWN)
    session.key("\n")
    state(4, "reset", 2)
    session.call("click_element", elementHandle=target, button="Right")
    session.call("click_element", elementHandle=item(session, context, "Unavailable"))
    state(4, "reset", 2)
    session.key(ESCAPE)
    check(not matching(session, context, "Reset to default"), "Escape must dismiss the native context menu")
    session.key("\u0010", "Press")
    session.key("\uf70d")
    session.key("\u0010", "Release")
    check(bool(matching(session, context, "Reset to default")), "Shift F10 must open the focused child's context menu")
    session.key(ESCAPE)
    state(4, "reset", 2)

    hover = session.find("MenuTests::hover-button")
    tooltip_report = session.find("MenuTests::tooltip-report")
    session.call("hover_element", elementHandle=button)
    session.wait_until(lambda: session.properties(tooltip_report)["accessibleLabel"] == "Tooltip: closed")
    session.call("hover_element", elementHandle=hover)
    time.sleep(.15)
    check(session.properties(tooltip_report)["accessibleLabel"] == "Tooltip: closed", "Tooltip must honor its hover delay")
    session.wait_until(lambda: session.properties(tooltip_report)["accessibleLabel"] == "Tooltip: open")
    tips = [element for element in session.tree()["elements"]
            if any(entry.get("typeName") == "Tooltip" for entry in element.get("typeNamesAndIds", []))]
    check(len(tips) == 1, "Hovering a nested interactive button must reveal one tooltip")
    tip = tips[0]
    x, y = tip["absolutePosition"].get("x", 0), tip["absolutePosition"].get("y", 0)
    window = session.properties(session.root)["size"]
    check(x >= 0 and y >= 0 and x + tip["size"]["width"] <= window["width"]
          and y + tip["size"]["height"] <= window["height"], "Tooltip must stay inside the supplied viewport")
    session.screenshot("menus-tooltip.png")
    session.call("click_element", elementHandle=hover)
    state(5, "hover", 2)
    session.call("hover_element", elementHandle=button)
    session.wait_until(lambda: session.properties(tooltip_report)["accessibleLabel"] == "Tooltip: closed")
    session.screenshot("menus-interactions.png")
    return f"PASS: {checks} independent dropdown, native context menu and delayed tooltip checks"
