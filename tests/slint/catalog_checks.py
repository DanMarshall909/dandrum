"""Input smoke checks of the composed catalog, including its real state picker."""
import math
import time
from menu_checks import elements, item, matching
from layout_gallery_checks import layout_gallery_geometry


def catalog(session, expected_size=(1200, 800)):
    checks = 0

    def check(ok, reason):
        nonlocal checks
        checks += 1
        if not ok:
            raise AssertionError(reason)

    def button(label):
        return item(session, session.root, label, "Button")

    def numeric(handle):
        return float(session.properties(handle)["accessibleValue"])

    def event(expected):
        session.wait_until(lambda: bool(matching(session, session.root, "Catalog event: " + expected)))

    def choose_state(label, value):
        session.call("click_element", elementHandle=item(session, session.root, "STATE", "Button"))
        session.call("click_element", elementHandle=button(label))
        event("Showing " + value + " state where applicable")
        check(bool(matching(session, session.root, value)), "The catalog state picker must retain its selected value")

    def navigate(label, identifier, heading):
        session.call("click_element", elementHandle=button(label))
        event("Page: " + identifier)
        check(bool(matching(session, session.root, heading)), f"Navigation to {label!r} must show its page heading")

    def outer_scrollbar():
        bars = [element for element in session.tree()["elements"]
                if element.get("accessibleRole") == "Slider" and element.get("accessibleLabel") == "Vertical scroll"]
        if not bars:
            raise AssertionError("A catalog page taller than its viewport must expose a vertical scrollbar")
        return max(bars, key=lambda element: element["absolutePosition"].get("x", 0))["handle"]

    def content_clip():
        viewport = session.find("CatalogWindow::viewport")
        return next(element for element in elements(session, viewport)
                    if any(entry.get("typeName") == "Flickable" for entry in element.get("typeNamesAndIds", [])))

    def popup_bounds(control, label):
        session.wait_until(lambda: bool(matching(session, control, label + " value popup", "Groupbox")))
        # Let a previous hover popup complete its 250ms dismissal grace period.
        time.sleep(.3)
        popup_handle = item(session, control, label + " value popup", "Groupbox")
        popup = session.properties(popup_handle)
        clip = content_clip()
        x, y = popup["absolutePosition"]["x"], popup["absolutePosition"]["y"]
        left, top = clip["absolutePosition"]["x"], clip["absolutePosition"]["y"]
        check(x >= left and y >= top and x + popup["size"]["width"] <= left + clip["size"]["width"]
              and y + popup["size"]["height"] <= top + clip["size"]["height"],
              f"The {label} value popup must remain inside the catalog content viewport's clipping bounds")
        descendants = elements(session, popup_handle)
        arrows = [element for element in descendants
                  if any(entry.get("id") == "ValuePopup::arrow" for entry in element.get("typeNamesAndIds", []))]
        if len(arrows) != 1:
            raise AssertionError(f"Expected one {label} popup arrow, found {len(arrows)}")
        arrow_center = arrows[0]["absolutePosition"]["x"] + arrows[0]["size"]["width"] / 2
        owner = session.properties(control)
        control_center = owner["absolutePosition"]["x"] + owner["size"]["width"] / 2
        check(math.isclose(arrow_center, control_center, abs_tol=.05),
              f"The {label} popup arrow must point to its control: expected x{control_center}, got x{arrow_center}")

    window = session.properties(session.root)["size"]
    check((window["width"], window["height"]) == expected_size,
          f"The catalog must run at its requested {expected_size[0]}×{expected_size[1]} size")
    level = item(session, session.root, "LEVEL", "Slider")
    check(numeric(level) == -3, "The catalog must render the supplied actual level")
    session.call("click_element", elementHandle=level)
    session.key("\u001b")
    session.key("\uf729")
    session.key("\uf700")
    check(math.isclose(numeric(level), -56.7, abs_tol=.011), "The composed catalog must deliver keyboard changes to its model")
    event("Level → -56.7")
    for label, state in [("Disabled", "disabled"), ("Prepared", "prepared")]:
        choose_state(label, state)
        before = numeric(level)
        session.call("set_element_value", elementHandle=level, value="-12")
        check(numeric(level) == before, f"The {label.lower()} catalog state must reject parameter edits")
        if state == "disabled":
            check(not session.properties(level).get("accessibleEnabled", False), "Disabled controls must expose their disabled state")
    choose_state("Host automated", "host")
    session.call("click_element", elementHandle=level)
    session.key("\u001b")
    session.key("\uf700")
    check(math.isclose(numeric(level), -53.4, abs_tol=.011), "Host automation styling must preserve local control input")
    choose_state("Default", "default")
    session.call("hover_element", elementHandle=level)
    popup_bounds(level, "LEVEL")
    session.screenshot("catalog-controls-interactions.png")
    free = item(session, session.root, "FREE", "Slider")
    session.call("hover_element", elementHandle=free)
    popup_bounds(free, "FREE")
    session.screenshot("catalog-free-popup.png")
    send = item(session, session.root, "SEND", "Slider")
    geometry, clip = session.properties(send), content_clip()
    overflow = geometry["absolutePosition"]["y"] + geometry["size"]["height"] + 8 - clip["absolutePosition"]["y"] - clip["size"]["height"]
    if overflow > 0:
        session.call("scroll_element", elementHandle=outer_scrollbar(), deltaY=-overflow)
        session.wait_until(lambda: session.properties(send)["absolutePosition"]["y"] + geometry["size"]["height"]
                           < clip["absolutePosition"]["y"] + clip["size"]["height"])
    session.call("click_element", elementHandle=send)
    session.key("\n")
    popup_bounds(send, "SEND")
    session.screenshot("catalog-slider-popup.png")
    session.key("\u001b")

    navigate("Pads and key map", "mapping", "Notes & velocity")
    layout = session.find("CatalogWindow::page-layout")
    before_y = session.properties(layout)["absolutePosition"].get("y", 0)
    bar = outer_scrollbar()
    geometry = session.properties(bar)
    window = session.properties(session.root)["size"]
    check(geometry["absolutePosition"]["x"] + geometry["size"]["width"] <= window["width"],
          "The vertical scrollbar's input bounds must fit inside the catalog window")
    session.call("scroll_element", elementHandle=bar, deltaY=-220)
    check(math.isclose(numeric(bar), 220, abs_tol=.011), "The catalog scrollbar must consume wheel input")
    session.wait_until(lambda: math.isclose(before_y - session.properties(layout)["absolutePosition"].get("y", 0),
                                            220, abs_tol=.011))
    after_y = session.properties(layout)["absolutePosition"].get("y", 0)
    check(math.isclose(before_y - after_y, 220, abs_tol=.011), "Wheel scrolling must move the actual page content")
    session.screenshot("catalog-mapping-scrolled.png")

    navigate("Waveforms", "waveforms", "Waveforms & analysis")
    check(numeric(outer_scrollbar()) == 0, "Changing pages must reset the scroll offset")
    primary = session.find("WaveformsPage::primary-waveform")
    for label, state, expected in [("Empty", "empty", "No sample selected"),
                                   ("Loading / rebuilding", "loading", "Loading sample…"),
                                   ("Error / unavailable", "error", "Sample analysis failed")]:
        choose_state(label, state)
        check(bool(matching(session, primary, expected)),
              f"Catalog state {label!r} must update the primary source display to {expected!r}")
        session.screenshot(f"catalog-state-{label.split(' ')[0].lower()}.png")
    session.call("click_element", elementHandle=item(session, primary, "Retry", "Button"))
    event("Source analysis retry requested")
    checks += 1
    choose_state("Default", "default")
    check(bool(matching(session, primary, "advanced-break.wav")), "Returning to default must restore the source's real label")
    session.screenshot("catalog-waveforms-interactions.png")

    navigate("Layers and output", "routing", "Sources, routing & levels")
    session.screenshot("catalog-routing-interactions.png")
    navigate("Layout and feedback", "layout", "Structure and feedback")
    panels = [element for element in session.tree()["elements"]
              if any(entry.get("typeName") == "Panel" for entry in element.get("typeNamesAndIds", []))]
    panel = panels[0]["handle"]
    initial_height = session.properties(panel)["size"]["height"]
    session.call("click_element", elementHandle=button("Reload sample"))
    event("Header action: reload")
    check(session.properties(panel)["size"]["height"] == initial_height, "Header actions must keep the composed panel open")
    session.call("click_element", elementHandle=button("Sample properties"))
    event("Sample panel collapsed")
    check(session.properties(panel)["size"]["height"] == 28, "The composed panel must collapse to its header")
    session.key(" ")
    event("Sample panel expanded")
    check(session.properties(panel)["size"]["height"] == initial_height, "Space must expand the focused catalog panel")
    session.call("click_element", elementHandle=outer_scrollbar())
    session.key("\uf72b")
    check(numeric(outer_scrollbar()) > 0, "End must expose the lower feedback gallery")
    checks += layout_gallery_geometry(session)
    session.screenshot("catalog-layout-feedback.png")
    navigate("Foundations", "foundations", "Foundations")
    check(numeric(outer_scrollbar()) == 0, "Navigation after a keyboard scroll must return to the top")
    session.screenshot("catalog-foundations-interactions.png")
    navigate("Controls", "controls", "Control language")
    check(math.isclose(numeric(item(session, session.root, "LEVEL", "Slider")), -53.4, abs_tol=.011),
          "Navigating away and back must retain the catalog's accepted control value")
    return f"PASS: {checks} independent catalog navigation, state, input, scrolling and collapse checks"
