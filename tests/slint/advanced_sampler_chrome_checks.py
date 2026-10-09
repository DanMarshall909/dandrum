"""Exercise native actions and measured v3 frame geometry on a private display."""


def header_controls(session):
    report = session.find("AdvancedSamplerChromeTest::action-report")
    checks = 0
    for name, expected in [("undo-button", "undo"), ("redo-button", "undo"),
                           ("patch-button", "patch.browse"),
                           ("diagnostics-button", "diagnostics"),
                           ("perform-button", "perform.toggle"),
                           ("appearance-button", "appearance")]:
        session.call("click_element", elementHandle=session.find("EditorHeader::" + name))
        assert session.properties(report)["accessibleLabel"] == expected, name
        checks += 1
    session.call("click_element", elementHandle=session.find("InstrumentTree::add-group-button"))
    assert session.properties(report)["accessibleLabel"] == "tree.add:"
    checks += 1
    session.call("click_element", elementHandle=session.find("Inspector::reset-overrides-button"))
    assert session.properties(report)["accessibleLabel"] == "inherit.reset:kick"
    checks += 1
    header = session.properties(session.find("AdvancedSamplerChromeTest::header"))
    assert header["absolutePosition"].get("y", 0) == 0
    assert header["size"]["height"] == 45
    checks += 2
    session.screenshot("advanced-header-input.png")
    return f"PASS: {checks} independent headless chrome checks"


def frame_geometry(session, width, height, compact=False):
    expected = (88, 43, 492, 423) if compact else (288, 49, 632, 373) if width == 1200 else (328, 49, 944, 505)
    frame = session.properties(session.find("AdvancedSamplerSceneTest::frame"))
    assert frame["size"] == {"width": width, "height": height}
    workspace = session.properties(session.find("EditorFrame::workspace"))
    actual = (workspace["absolutePosition"].get("x", 0), workspace["absolutePosition"].get("y", 0), workspace["size"]["width"], workspace["size"]["height"])
    assert actual == expected, (actual, expected)
    session.screenshot(f"advanced-frame-{width}x{height}.png")
    return "PASS: native workspace geometry matches measured v3 bounds"


def waveform_controls(session):
    import math
    session.call("click_element", elementHandle=session.find("AdvancedSamplerWaveTest::run"))
    labels = [session.properties(e["handle"]).get("accessibleLabel", "") for e in session.tree()["elements"]]
    assert "PASS: 4 source coordinate contracts" in labels
    start = session.find("WaveView::start-handle")
    bounds = session.properties(start)
    x = bounds["absolutePosition"]["x"] + 6
    y = bounds["absolutePosition"]["y"] + 50
    session.call("drag_element", elementHandle=start, target={"x": x + 60, "y": y})
    fields = session.properties(session.find("AdvancedSamplerWaveTest::events"))["accessibleLabel"].split(":")
    assert fields[0] == "region-start" and fields[2:] == ["1", "1"], fields
    assert math.isclose(float(fields[1]), .3, abs_tol=.01), fields
    session.screenshot("advanced-wave-input.png")
    return "PASS: source coordinate contracts and balanced pointer gesture"
