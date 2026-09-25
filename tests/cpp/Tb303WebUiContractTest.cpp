#include "Tb303WebUi.h"

#include <fstream>
#include <iostream>
#include <sstream>
#include <string_view>

int main()
{
    const std::string_view html { Tb303WebUi::indexHtml };

    if (html.find ("const specs=") != std::string_view::npos)
    {
        std::cerr << "web UI still declares a hard-coded parameter surface\n";
        return 1;
    }

    if (html.find ("renderParameters") == std::string_view::npos
        || html.find ("parameterValuesChanged") == std::string_view::npos)
    {
        std::cerr << "web UI does not render and update the processor's active parameters\n";
        return 1;
    }

    if (html.find ("native('noteOn')") == std::string_view::npos
        || html.find ("native('noteOff')") == std::string_view::npos)
    {
        std::cerr << "web UI keyboard does not call the native note bridge\n";
        return 1;
    }

    if (html.find ("native('renderSoundLab')") == std::string_view::npos
        || html.find ("native('getSoundLabAnalysis')") == std::string_view::npos
        || html.find ("soundLabAnalysisChanged") == std::string_view::npos)
    {
        std::cerr << "web UI does not use the native Sound Lab state bridge\n";
        return 1;
    }

    if (html.find ("id=\"soundLabPlot\"") == std::string_view::npos
        || html.find ("id=\"soundLabAudio\"") == std::string_view::npos
        || html.find ("spectral_centroid_hz") == std::string_view::npos)
    {
        std::cerr << "web UI does not present Sound Lab audio and spectral trajectories\n";
        return 1;
    }

    if (html.find ("native('chooseSoundLabReference')") == std::string_view::npos
        || html.find ("native('matchSoundLab')") == std::string_view::npos
        || html.find ("native('cancelSoundLab')") == std::string_view::npos
        || html.find ("native('acceptSoundLabMatch')") == std::string_view::npos
        || html.find ("native('requestGraphProposal')") == std::string_view::npos)
    {
        std::cerr << "web UI does not expose the reference/match/accept/proposal workflow\n";
        return 1;
    }

    if (html.find ("id=\"soundLabProgress\"") == std::string_view::npos
        || html.find ("id=\"soundLabScore\"") == std::string_view::npos
        || html.find ("id=\"soundLabReferenceAudio\"") == std::string_view::npos
        || html.find ("id=\"soundLabCandidateAudio\"") == std::string_view::npos
        || html.find ("comparison_metrics") == std::string_view::npos
        || html.find ("best_parameters") == std::string_view::npos)
    {
        std::cerr << "web UI does not show matching progress, A/B audio, scores, and comparison data\n";
        return 1;
    }

    std::ifstream editorSourceFile (DANDRUM_SOURCE_ROOT "/src/juce-plugin/PluginEditor.cpp");
    std::ostringstream editorSourceBuffer;
    editorSourceBuffer << editorSourceFile.rdbuf();
    const auto editorSource = editorSourceBuffer.str();
    if (! editorSourceFile
        || editorSource.find ("\"renderSoundLab\"") == std::string::npos
        || editorSource.find ("\"getSoundLabAnalysis\"") == std::string::npos
        || editorSource.find ("\"chooseSoundLabReference\"") == std::string::npos
        || editorSource.find ("\"matchSoundLab\"") == std::string::npos
        || editorSource.find ("\"cancelSoundLab\"") == std::string::npos
        || editorSource.find ("\"acceptSoundLabMatch\"") == std::string::npos
        || editorSource.find ("\"requestGraphProposal\"") == std::string::npos
        || editorSource.find ("\"soundLabAnalysisChanged\"") == std::string::npos
        || editorSource.find ("\"/sound-lab.wav\"") == std::string::npos
        || editorSource.find ("\"/sound-lab-reference.wav\"") == std::string::npos
        || editorSource.find ("\"/sound-lab-candidate.wav\"") == std::string::npos
        || editorSource.find ("reloadInstrumentFromFile") == std::string::npos)
    {
        std::cerr << "JUCE editor does not register the Sound Lab bridge and WAV resource\n";
        return 1;
    }

    if (editorSource.find ("snapshot.state == SoundLabController::State::cancelled")
        != std::string::npos)
    {
        std::cerr << "JUCE editor rejects the retained best candidate from a cancelled match\n";
        return 1;
    }

    return 0;
}
