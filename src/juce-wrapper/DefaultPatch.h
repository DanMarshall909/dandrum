#pragma once

#include <filesystem>
#include <string>

namespace dandrum
{
// Search upward from the current working directory so the binary works from
// either the repo root or the CTest/build tree without hard-coding paths.
inline std::filesystem::path defaultPatchPath()
{
    const auto relativePath = std::filesystem::path ("examples/patches/polyphonic-pad.yaml");
    auto directory = std::filesystem::current_path();

    for (int i = 0; i < 6; ++i)
    {
        const auto candidate = directory / relativePath;

        if (std::filesystem::exists (candidate))
            return candidate;

        if (! directory.has_parent_path())
            break;

        directory = directory.parent_path();
    }

    return relativePath;
}
} // namespace dandrum
