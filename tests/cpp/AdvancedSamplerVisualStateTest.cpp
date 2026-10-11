#include "ui/advanced-sampler/engine/Model.h"
#include <cmath>
#include <iostream>
#include <stdexcept>

int main() {
    int checks = 0;
    const auto require = [&](bool ok, const char* message) {
        ++checks;
        if (!ok) throw std::runtime_error(message);
    };
    try {
        using dandrum::sampler::Model;
        Model model;
        require(model.selectState("empty") && model.text("scope") == "instrument",
                "Empty instrument fixture uses the complete instrument scope");
        require(model.selectState("overview") && model.text("scope") == "group",
                "Overview fixture uses the actual selected group's scope");
        require(model.selectState("sample"), "Return to the selected sample fixture");
        const auto navigationHistory = model.undoCount();
        require(model.command("parameter.navigate", "cutoff") && model.text("page") == "voice"
                && model.undoCount() == navigationHistory, "Destination navigation changes workspace without an edit");
        require(!model.command("parameter.navigate", "unknown") && model.text("page") == "voice"
                && model.undoCount() == navigationHistory, "Unknown destination preserves workspace and history");
        require(model.selectState("mapping"), "Mapping fixture loads");
        require(model.text("selected-zone") == "sl0", "Guide mapping selects Kick 1 at note 24");
        require(model.selectState("sample")
                && model.find("zone", model.text("selected-zone"))->text("source") == model.text("selected-source"),
                "Sample fixture selects a zone belonging to its actual selected source");
        const auto priorSource = *model.find("source", model.text("selected-source"));
        require(model.selectState("unsupported") && model.text("asset-state") == "unsupported",
                "Rejected-format fixture exposes its import error");
        require(model.find("source", model.text("selected-source"))->fields == priorSource.fields
                && model.peaks().size() == 600,
                "Rejecting a format in the design fixture preserves the actual prior cached source and waveform");
        require(model.selectState("slices") && model.peaks().size() == 400,
                "Break fixture exposes the reference's own 400 source peaks");
        require(std::abs(model.peaks()[0].maximum - 0.9369740108726545) < 1e-6,
                "Break first signed transient matches the reference's seeded waveform");
        require(model.peaks()[0].minimum == -model.peaks()[0].maximum,
                "Break waveform preserves signed extrema");
        require(model.selectState("vel"), "Velocity fixture loads");
        require(model.text("selected-zone") == "snH" && model.number("crossfade-width") == 0,
                "Guide velocity fixture selects Hard with crossfade off");
        require(model.selectState("routing"), "Routing fixture loads");
        require(model.find("tree", "patch")->text("name") == "Expressive Kit",
                "Guide root preserves its actual patch name");
        require(model.find("bus", "reverb")->text("routing-kind") == "send"
                && model.find("bus", "main")->text("routing-kind") == "output",
                "Routing distinguishes plugin outputs from retained FX sends");
        require(std::abs(model.find("bus", "main")->number("meter-l") - std::pow(10., -14. / 20.)) < 1e-9
                && std::abs(model.find("bus", "main")->number("meter-r") - std::pow(10., -15. / 20.)) < 1e-9,
                "Guide stereo meters retain independent signed decibel measurements as linear amplitudes");
        require(model.outputFor("snare") == "Snare 5/6" && model.outputFor("kick") == "Drums 3/4",
                "Guide routing preserves explicit Snare and inherited Drums output");
        const auto* reverb = model.find("send", "snare:Reverb");
        require(reverb && std::abs(reverb->number("value") - std::pow(10., -10.5 / 20.)) < 1e-9,
                "Guide Snare send retains the reference decibel value as linear gain");
        require(model.selectState("running") && model.text("analysis-state") == "running"
                && model.number("analysis-progress") == .48, "Running analysis fixture starts at 48 percent");
        require(model.selectState("freeze") && model.text("freeze-state") == "confirm",
                "Freeze fixture opens confirmation before rendering");
        require(!model.find("source", model.text("selected-source"))->number("frozen"),
                "Opening Freeze confirmation does not freeze the source");
        require(model.selectState("diag"), "Diagnostics fixture loads");
        bool hit = false;
        for (const auto& event : model.records("event")) hit |= event.text("kind") == "note-on";
        require(!hit && model.heldNotes().empty(), "Diagnostics begins with no synthetic hit or held note");
        for (const auto* owner : {"drums", "reverb"}) {
            require(model.selectState("fx"), "Effects fixture loads");
            const auto table = std::string(owner) == "drums" ? "tree" : "bus";
            const auto original = model.find(table, owner)->number("gain");
            require(model.command("chain.gain", owner, "", -4.5), "Group and bus chain gain controls work");
            require(model.find(table, owner)->number("gain") == -4.5, "Chain gain persists on its owner");
            require(model.command("undo") && model.find(table, owner)->number("gain") == original,
                    "Chain gain Undo restores exactly the prior owner value");
            require(model.gesture(std::string("chain:") + owner + ":gain", 0, original), "Chain gain gesture starts");
            require(model.gesture(std::string("chain:") + owner + ":gain", 1, -3), "Chain gain gesture updates");
            require(model.gesture(std::string("chain:") + owner + ":gain", 3, -3)
                    && model.find(table, owner)->number("gain") == original, "Cancelled chain gesture restores owner");
            require(model.command("chain.mute", owner) && model.find(table, owner)->number("muted") == 1,
                    "Group and bus chain mute controls work");
            require(model.command("undo") && model.find(table, owner)->number("muted") == 0,
                    "Chain mute Undo restores the owner");
            const auto output = model.find(table, owner)->text("output");
            const auto destination = output == "Main 1/2" ? "Drums 3/4" : "Main 1/2";
            require(model.command("chain.output", owner, destination)
                    && model.find(table, owner)->text("output") == destination, "Chain output control persists");
            require(model.command("undo") && model.find(table, owner)->text("output") == output,
                    "Chain output Undo restores prior output");
        }
        const auto history = model.undoCount();
        require(!model.command("chain.gain", "missing", "", 2) && model.undoCount() == history,
                "Unknown chain owners cannot change history");
        require(!model.command("chain.output", "drums", "Unknown bus") && model.undoCount() == history,
                "Unknown chain output is rejected without an edit");
        std::cout << "PASS: " << checks << " visual-state and live chain checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
