#pragma once
#include <filesystem>
#include <optional>
#include <string_view>
#include <vector>

namespace dandrum::sampler {
// A desktop selection is accepted atomically: invalid or remote entries reject
// the entire selection. The host remains responsible for opening these files.
std::optional<std::vector<std::filesystem::path>> decode_file_uris(std::string_view selection);
}
