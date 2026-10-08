"""Rendered content/bar placement in one-axis and two-axis scroll viewports."""
import math
from menu_checks import elements


def scroll_geometry(session):
    failures, checks = [], 0

    def check(ok, reason):
        nonlocal checks
        checks += 1
        if not ok:
            failures.append(reason)

    def right(element):
        return element["absolutePosition"]["x"] + element["size"]["width"]

    def bottom(element):
        return element["absolutePosition"]["y"] + element["size"]["height"]

    for name in ["vertical", "horizontal", "both"]:
        handle = session.find(f"LayoutTests::{name}-viewport")
        parent = session.properties(handle)
        children = elements(session, handle)
        clip = next(element for element in children
                    if any(entry.get("typeName") == "Flickable" for entry in element.get("typeNamesAndIds", [])))
        check(all(math.isclose(clip["absolutePosition"][axis], parent["absolutePosition"][axis], abs_tol=.01)
                  for axis in ["x", "y"]), f"{name} viewport content must begin at the viewport origin")
        vertical = next((element for element in children if element.get("accessibleRole") == "Slider"
                         and element.get("accessibleLabel") == "Vertical scroll"), None)
        horizontal = next((element for element in children if element.get("accessibleRole") == "Slider"
                           and element.get("accessibleLabel") == "Horizontal scroll"), None)
        if (vertical is not None, horizontal is not None) != (name != "horizontal", name != "vertical"):
            raise AssertionError(f"The {name} fixture must render its configured scrollbar axes")
        if vertical:
            check(math.isclose(vertical["absolutePosition"]["y"], parent["absolutePosition"]["y"], abs_tol=.01),
                  f"{name} vertical scrollbar must begin at the viewport top")
            check(right(clip) <= vertical["absolutePosition"]["x"], f"{name} content must not overlap its vertical scrollbar")
        if horizontal:
            check(math.isclose(horizontal["absolutePosition"]["x"], parent["absolutePosition"]["x"], abs_tol=.01),
                  f"{name} horizontal scrollbar must begin at the viewport left")
            check(bottom(clip) <= horizontal["absolutePosition"]["y"], f"{name} content must not overlap its horizontal scrollbar")
        if vertical and horizontal:
            check(bottom(vertical) <= horizontal["absolutePosition"]["y"]
                  and right(horizontal) <= vertical["absolutePosition"]["x"],
                  "Two-axis scrollbar bodies must leave their shared corner clear")
    if failures:
        session.screenshot("scroll-gutter-failure.png")
        raise AssertionError("; ".join(failures))
    return checks
