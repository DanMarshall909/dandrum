#pragma once
#include <slint.h>
#include <chrono>
#include <functional>
#include <memory>
#include <string>
#include "FileDropProtocol.h"
#ifdef DANDRUM_SLINT_HAVE_X11_FILE_DROP
#include "X11FileDrop.h"
#endif

namespace dandrum::slint_ui {
// Retain for the editor's lifetime. Polling and delivery run on the UI thread;
// weak ownership permits the window to disappear before the transport owner.
// The caller chooses how its own window consumes a local file selection.
template<class Window> class WindowFileDrop {
#ifdef DANDRUM_SLINT_HAVE_X11_FILE_DROP
    X11FileDrop drop_;
    slint::Timer poll_;
#endif
public:
    using Received = std::function<void(const slint::ComponentHandle<Window>&, slint::DataTransfer)>;
    WindowFileDrop(const slint::ComponentHandle<Window>& window, std::string title, Received received)
#ifdef DANDRUM_SLINT_HAVE_X11_FILE_DROP
        : drop_(std::move(title), [weak = slint::ComponentWeakHandle<Window>(window), received = std::move(received)](auto paths) {
            if (const auto live = weak.lock()) {
                slint::DataTransfer transfer;
                transfer.set_file_paths(std::move(paths));
                received(*live, std::move(transfer));
            }
        })
#endif
    {
#ifdef DANDRUM_SLINT_HAVE_X11_FILE_DROP
        poll_.start(slint::TimerMode::Repeated, file_drop::poll_interval, [this] { drop_.poll(); });
#else
        (void)window; (void)title; (void)received;
#endif
    }
    bool active() const {
#ifdef DANDRUM_SLINT_HAVE_X11_FILE_DROP
        return drop_.active();
#else
        return false;
#endif
    }
};

template<class Window, class Received>
auto bind_window_file_drop(const slint::ComponentHandle<Window>& window, std::string title, Received received) {
    return std::make_unique<WindowFileDrop<Window>>(window, std::move(title), std::move(received));
}
}
