#pragma once
#include "../../slint/platform/WindowFileDrop.h"

namespace dandrum::sampler {
// Sampler file policy remains in the adapter; the transport only delivers files.
template<class Window> class WindowFileDrop : public dandrum::slint_ui::WindowFileDrop<Window> {
public:
    WindowFileDrop(const slint::ComponentHandle<Window>& window, std::string title)
        : dandrum::slint_ui::WindowFileDrop<Window>(window, std::move(title), [](const auto& live, slint::DataTransfer transfer) {
            live->invoke_receive_files(live->invoke_file_paths(std::move(transfer)));
        }) { }
};

template<class Window>
auto bind_native_file_drop(const slint::ComponentHandle<Window>& window,
        std::string title = "Dandrum Advanced Sampler · silent editor") {
    return std::make_unique<WindowFileDrop<Window>>(window, std::move(title));
}
}
