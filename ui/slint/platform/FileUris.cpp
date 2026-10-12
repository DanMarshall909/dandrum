#include "FileUris.h"
#include "FileDropProtocol.h"
#include <string>

namespace dandrum::slint_ui {
std::optional<std::vector<std::filesystem::path>> decode_file_uris(std::string_view selection) {
    if (selection.size() > file_drop::selection_bytes) return std::nullopt;
    std::vector<std::filesystem::path> files;
    while (!selection.empty()) {
        const auto newline = selection.find('\n');
        auto line = selection.substr(0, newline);
        selection = newline == std::string_view::npos ? std::string_view{} : selection.substr(newline + 1);
        if (line.ends_with('\r')) line.remove_suffix(1);
        if (line.empty() || line.front() == '#') continue;
        if (!line.starts_with(file_drop::uri_prefix)) return std::nullopt;
        line.remove_prefix(file_drop::uri_prefix.size());
        if (line.starts_with(file_drop::local_authority) && line.size() > file_drop::local_authority.size()
                && line[file_drop::local_authority.size()] == '/') line.remove_prefix(file_drop::local_authority.size());
        if (line.empty() || line.front() != '/' || files.size() == file_drop::maximum_files) return std::nullopt;
        std::string path;
        auto hex = [](char c) -> int {
            if (c >= '0' && c <= '9') return c - '0';
            if (c >= 'a' && c <= 'f') return c - 'a' + 10;
            if (c >= 'A' && c <= 'F') return c - 'A' + 10;
            return -1;
        };
        for (std::size_t i = 0; i < line.size(); ++i) {
            unsigned char c = line[i];
            if (c == '?' || c == '#') return std::nullopt;
            if (c == '%') {
                if (i + 2 >= line.size()) return std::nullopt;
                const auto high = hex(line[i + 1]), low = hex(line[i + 2]);
                if (high < 0 || low < 0) return std::nullopt;
                c = static_cast<unsigned char>(high * 16 + low); i += 2;
            }
            if (c < 32 || c == 127 || path.size() == file_drop::path_bytes) return std::nullopt;
            path.push_back(static_cast<char>(c));
        }
        files.emplace_back(path);
    }
    if (files.empty()) return std::nullopt;
    return files;
}
}
