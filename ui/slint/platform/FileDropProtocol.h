#pragma once
#include <array>
#include <chrono>
#include <cstddef>
#include <cstdint>
#include <string_view>

namespace dandrum::slint_ui::file_drop {
// Transport bounds and timings are policy, independent of theme geometry.
inline constexpr std::size_t selection_bytes = 65536;
inline constexpr std::size_t maximum_files = 256;
inline constexpr std::size_t path_bytes = 4096;
inline constexpr std::string_view uri_prefix = "file://";
inline constexpr std::string_view local_authority = "localhost";
inline constexpr auto poll_interval = std::chrono::milliseconds{20};
inline constexpr auto scan_interval = std::chrono::milliseconds{250};
inline constexpr auto selection_timeout = std::chrono::seconds{5};
inline constexpr unsigned discovery_windows = 256;
inline constexpr unsigned discovery_depth = 8;
inline constexpr unsigned replies_per_poll = 128;
inline constexpr unsigned events_per_poll = 128;
inline constexpr unsigned title_words = 256;
inline constexpr unsigned type_list_words = 64;
inline constexpr unsigned selection_words = selection_bytes / sizeof(std::uint32_t);
inline constexpr unsigned word_bits = 32;
inline constexpr unsigned byte_bits = 8;
inline constexpr unsigned version_shift = 24;
inline constexpr unsigned minimum_version = 3;
inline constexpr unsigned version = 5;
inline constexpr unsigned status_accept = 1;
inline constexpr unsigned status_want_positions = 2;
inline constexpr unsigned synthetic_event_mask = 127;
inline constexpr const char* display_variable = "DISPLAY";
enum AtomIndex : std::size_t {
    name, utf8, pid, proxy, aware, enter, position, leave, drop, status,
    finished, selection, uri, types, copy, data
};
inline constexpr std::array atom_names {
    "_NET_WM_NAME", "UTF8_STRING", "_NET_WM_PID", "XdndProxy", "XdndAware",
    "XdndEnter", "XdndPosition", "XdndLeave", "XdndDrop", "XdndStatus",
    "XdndFinished", "XdndSelection", "text/uri-list", "XdndTypeList",
    "XdndActionCopy", "_DANDRUM_XDND_DATA"
};
}
