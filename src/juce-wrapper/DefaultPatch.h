#pragma once

#include <filesystem>
#include <string>

namespace dandrum
{
inline constexpr auto defaultPatchRelativePath = "examples/patches/synthetic-808-kick.yaml";
inline constexpr auto defaultDrumContainerRelativePath = "examples/patches/drum-kit.yaml";
inline constexpr auto soundDesignFixtureRelativePath = "examples/sound-design/tb303-acid-poc.yaml";

inline std::filesystem::path findRepositoryExample (const std::filesystem::path& relativePath)
{
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

// Search upward from the current working directory so the binary works from
// either the repo root or the CTest/build tree without hard-coding paths.
inline std::filesystem::path defaultPatchPath()
{
    return findRepositoryExample (defaultPatchRelativePath);
}

inline std::filesystem::path defaultDrumContainerPath()
{
    return findRepositoryExample (defaultDrumContainerRelativePath);
}

inline std::filesystem::path soundDesignFixturePath()
{
    return findRepositoryExample (soundDesignFixtureRelativePath);
}
} // namespace dandrum
