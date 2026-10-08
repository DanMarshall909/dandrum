"""Geometry contracts for composed layout examples at either catalog width."""
import math
from menu_checks import elements, item


def layout_gallery_geometry(session):
    def component(parent, name):
        matches = [element for element in elements(session, parent)
                   if any(entry.get("typeName") == name for entry in element.get("typeNamesAndIds", []))]
        if len(matches) != 1:
            raise AssertionError(f"Expected one {name}, found {len(matches)}")
        return matches[0]

    def right(element):
        return element["absolutePosition"].get("x", 0) + element["size"]["width"]

    def bottom(element):
        return element["absolutePosition"].get("y", 0) + element["size"]["height"]

    checks, failures = 0, []

    def check(ok, reason):
        nonlocal checks
        checks += 1
        if not ok:
            failures.append(reason)

    bars = [element for element in elements(session, session.root)
            if element.get("accessibleRole") == "Slider" and element.get("accessibleLabel") == "Vertical scroll"]
    bar = max(bars, key=lambda element: element["absolutePosition"].get("x", 0))["handle"] if bars else None
    if bar:
        session.call("click_element", elementHandle=bar)
        session.key("\uf729")
    containers = component(session.root, "ContainersGallery")
    heading = session.properties(item(session, containers["handle"], "SOURCE SELECTION"))
    check(heading["size"]["height"] <= 24, "A source section heading must not absorb the gallery's spare height")
    if bar:
        session.call("scroll_element", elementHandle=bar, deltaY=-226)
    navigation = component(session.root, "NavigationGallery")
    panel = component(navigation["handle"], "Panel")
    target = session.properties(item(session, panel["handle"], "Right click for actions", "Button"))
    help_text = session.properties(item(session, panel["handle"],
        "Arrow keys skip unavailable choices. Enter selects; Escape closes menus."))
    check(bottom(target) <= bottom(panel) - 12 and bottom(help_text) <= bottom(panel) - 12,
          "Navigation controls and help must fit inside the expanded panel")
    if bar:
        session.key("\uf72b")
    feedback = component(session.root, "FeedbackGallery")
    empty = component(feedback["handle"], "EmptyState")
    choose = session.properties(item(session, empty["handle"], "Choose sample", "Button"))
    icon = component(empty["handle"], "Icon")
    check(empty["size"]["width"] >= 240, "The empty-state column must retain enough width at both catalog sizes")
    check(choose["absolutePosition"]["x"] >= empty["absolutePosition"]["x"] + 12
          and right(choose) <= right(empty) - 12, "The empty-state action must stay inside its padded surface")
    check(choose["size"]["width"] <= 144, "The empty-state action must retain its intrinsic compact width")
    check(math.isclose(icon["absolutePosition"]["x"] + icon["size"]["width"] / 2,
                       empty["absolutePosition"]["x"] + empty["size"]["width"] / 2, abs_tol=.011),
          "The empty-state icon must be centered above its text")
    if failures:
        session.screenshot("layout-gallery-geometry-failure.png")
        raise AssertionError("; ".join(failures))
    return checks
