#pragma once
#include <slint.h>
#include <memory>
#include <string>
#include <string_view>
#include <utility>

namespace dandrum::sampler {
template<class Window, class SessionBinding>
void bind_file_operations(const Window& window, const std::shared_ptr<SessionBinding>& binding)
{
    const std::weak_ptr<SessionBinding> weak(binding);
    using DroppedFiles = decltype(window->invoke_file_paths(std::declval<slint::DataTransfer>()));
    window->on_file_paths([weak](slint::DataTransfer transfer) {
        if (weak.expired()) return DroppedFiles{};
        const auto paths = transfer.file_paths();
        if (!paths || paths->empty() || paths->size() > 256) return DroppedFiles{};
        std::string text;
        for (const auto& path : *paths) {
            const auto value = path.string();
            if (value.empty() || value.find_first_of("\r\n") != std::string::npos) return DroppedFiles{};
            if (!text.empty()) text += '\n';
            text += value;
            if (text.size() > 65536) return DroppedFiles{};
        }
        DroppedFiles result{};
        result.paths = slint::SharedString(text.c_str());
        result.multiple = paths->size() > 1;
        return result;
    });
    window->on_file_operation([weak](slint::SharedString action, slint::SharedString path, slint::SharedString mode) {
        const auto live = weak.lock();
        if (!live) return false;
        const auto model = live->model();
        const std::string_view command(action.data(), action.size());
        const auto target = command == "asset.import-many" ? std::string(mode.data())
                : command.starts_with("asset.") ? model->text("selected-source") : "";
        const bool accepted = model->command(command, target, path.data(), 48);
        live->refresh();
        return accepted;
    });
}
}
