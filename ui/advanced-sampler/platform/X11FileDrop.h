#pragma once
#include <filesystem>
#include <functional>
#include <memory>
#include <string>
#include <vector>

namespace dandrum::sampler {
// Optional desktop adapter for Slint versions that do not forward winit's
// DroppedFile event. poll() belongs to the existing UI timer, never audio code.
// The implementation has its own X11 connection and does not read Slint events.
class X11FileDrop {
public:
    using Files = std::vector<std::filesystem::path>;
    using Received = std::function<void(Files)>;
    X11FileDrop(std::string exactWindowTitle, Received received);
    ~X11FileDrop();
    X11FileDrop(const X11FileDrop&) = delete;
    X11FileDrop& operator=(const X11FileDrop&) = delete;
    void poll();
    bool active() const;
private:
    struct State;
    std::unique_ptr<State> state_;
};
}
