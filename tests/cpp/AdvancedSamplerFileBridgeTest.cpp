#include "ui/advanced-sampler/engine/FileOperations.h"
#include "ui/advanced-sampler/engine/Model.h"
#include <functional>
#include <array>
#include <iostream>
#include <stdexcept>

struct DroppedFiles { slint::SharedString paths; bool multiple{}; };
struct FileProbe {
    mutable std::function<bool(slint::SharedString, slint::SharedString, slint::SharedString)> operation;
    mutable std::function<DroppedFiles(slint::DataTransfer)> file_paths;
    template<class F> void on_file_operation(F callback) const { operation = std::move(callback); }
    template<class F> void on_file_paths(F callback) const { file_paths = std::move(callback); }
    DroppedFiles invoke_file_paths(slint::DataTransfer transfer) const { return file_paths(std::move(transfer)); }
};
struct FileSession {
    std::shared_ptr<dandrum::sampler::Model> owned;
    int refreshes{};
    auto model() const { return owned; }
    void refresh() { ++refreshes; }
};
int main() {
    int checks = 0;
    const auto require = [&](bool ok, const char* description) {
        ++checks;
        if (!ok) throw std::runtime_error(description);
    };
    try {
        auto model = std::make_shared<dandrum::sampler::Model>();
        auto binding = std::make_shared<FileSession>(FileSession{model});
        auto window = std::make_shared<FileProbe>();
        dandrum::sampler::bind_file_operations(window, binding);
        require(bool(window->file_paths), "Native file transfer has a production callback");
        slint::DataTransfer transfer;
        transfer.set_file_paths({std::filesystem::path("/tmp/Felt C3.wav"), std::filesystem::path("/tmp/Felt C4.wav")});
        const auto transferUndo = model->undoCount();
        require(window->file_paths(transfer).paths == "/tmp/Felt C3.wav\n/tmp/Felt C4.wav"
                && window->file_paths(transfer).multiple,
                "Native file transfer preserves multiple paths and spaces for the chooser");
        require(model->undoCount() == transferUndo,
                "Inspecting a hovered native transfer does not create an import or history entry");
        require(window->file_paths(slint::DataTransfer("plain text")).paths.empty(),
                "A text drag is rejected by the sample file boundary");
        require(window->file_paths(slint::DataTransfer{}).paths.empty(), "An empty transfer is rejected");
        slint::DataTransfer single;
        single.set_file_paths({std::filesystem::path("/tmp/one.wav")});
        require(window->file_paths(single).paths == "/tmp/one.wav" && !window->file_paths(single).multiple,
                "A single native path bypasses the multi-file chooser");
        slint::DataTransfer invalid;
        invalid.set_file_paths({std::filesystem::path{}});
        require(window->file_paths(invalid).paths.empty(), "An empty file path rejects the whole transfer");
        invalid.set_file_paths(std::vector<std::filesystem::path>(257, "/tmp/sample.wav"));
        require(window->file_paths(invalid).paths.empty(), "Native import transfer count remains bounded");
        invalid.set_file_paths({std::filesystem::path(std::string(65537, 'a'))});
        require(window->file_paths(invalid).paths.empty(), "Native import transfer text remains bounded");
        slint::DataTransfer ambiguous;
        ambiguous.set_file_paths({std::filesystem::path("/tmp/ambiguous\nname.wav")});
        require(window->file_paths(ambiguous).paths.empty(), "Ambiguous newline file names cannot become extra imported paths");
        for (const auto* mode : {"sequential", "root-velocity", "stack", "round-robin"}) {
            model->command("patch.load", "kit");
            const auto before = model->records("zone").size();
            const auto undo = model->undoCount();
            require(window->operation("asset.import-many", "felt_C3_p.wav\nfelt_C3_f.wav", mode),
                    "Real file bridge forwards each selected mapping mode instead of a source id");
            require(model->records("zone").size() == before + 2 && model->undoCount() == undo + 1,
                    "Native multi-import creates two mappings in one undo entry");
            if (std::string_view(mode) == "sequential")
                require(model->records("zone").back().number("root-note") == 49,
                        "One-per-key import begins at the guide's C3 note");
            require(model->command("undo") && model->records("zone").size() == before,
                    "Undo removes the complete multi-import transaction");
        }
        const auto refreshes = binding->refreshes;
        require(!window->operation("asset.import-many", "felt.wav", "invalid") && binding->refreshes == refreshes + 1,
                "Invalid modes fail and refresh the observable native state");
        const auto selected = model->text("selected-source");
        require(window->operation("asset.replace", "replacement.wav", ""), "Replace resolves the selected source independently of mapping mode");
        require(model->find("source", selected)->text("path") == "replacement.wav", "Replace preserves the source identity");
        require(window->operation("asset.import", "additional.wav", ""), "Single-file import has its own native entry");
        require(model->text("selected-source") != selected, "Single-file import selects a new source");
        require(!window->operation("patch.import", "/a-directory-that-does-not-exist/missing.patch", ""),
                "Definition file failure remains observable through the same production bridge");
        const auto sources = model->records("source").size();
        binding.reset();
        require(window->file_paths(transfer).paths.empty(), "An expired file transfer callback rejects the native drop");
        require(!window->operation("asset.import", "expired.wav", "stack") && model->records("source").size() == sources,
                "An expired file-dialog callback cannot mutate a destroyed editor session");
        std::cout << "PASS: " << checks << " native file bridge checks\n";
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n';
        return 1;
    }
}
