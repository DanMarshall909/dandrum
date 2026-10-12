#pragma once
#include <slint.h>
#include <chrono>
#include <memory>
#include <string>
#ifdef DANDRUM_HAVE_X11_FILE_DROP
#include "X11FileDrop.h"
#endif

namespace dandrum::sampler {
// Retain alongside the session binding. Both polling and delivery stay on the
// Slint event loop; weak ownership permits the window to be destroyed first.
template<class Window> class WindowFileDrop {
#ifdef DANDRUM_HAVE_X11_FILE_DROP
    X11FileDrop drop_;
    slint::Timer poll_;
#endif
public:
    explicit WindowFileDrop(const slint::ComponentHandle<Window>& window,
                            std::string title)
#ifdef DANDRUM_HAVE_X11_FILE_DROP
        : drop_(std::move(title), [weak = slint::ComponentWeakHandle<Window>(window)](auto paths) {
            if (const auto live = weak.lock()) {
                slint::DataTransfer transfer;
                transfer.set_file_paths(std::move(paths));
                (*live)->invoke_receive_files((*live)->invoke_file_paths(std::move(transfer)));
            }
        })
#endif
    {
#ifdef DANDRUM_HAVE_X11_FILE_DROP
        poll_.start(slint::TimerMode::Repeated, std::chrono::milliseconds(20),
                    [this] { drop_.poll(); });
#else
        (void)window;
        (void)title;
#endif
    }
};

template<class Window>
auto bind_native_file_drop(const slint::ComponentHandle<Window>& window,
        std::string title = "Dandrum Advanced Sampler · silent editor") {
    return std::make_unique<WindowFileDrop<Window>>(window, std::move(title));
}
}
