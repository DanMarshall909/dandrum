#include "ui/slint/platform/FileUris.h"
#include <iostream>
#include <stdexcept>

int main() {
    using dandrum::slint_ui::decode_file_uris;
    int checks = 0;
    auto expect = [&](bool condition, const char* message) {
        ++checks; if (!condition) throw std::runtime_error(message);
    };
    try {
        const auto decoded = decode_file_uris("# desktop selection\r\n\r\nfile:///tmp/a%20b.wav\r\nfile://localhost/tmp/%E9%9F%B3+%23%3F.wav\n");
        expect(decoded && *decoded == std::vector<std::filesystem::path>{"/tmp/a b.wav", "/tmp/音+#?.wav"}, "Decode ordered local files, spaces, Unicode, plus and escaped reserved characters");
        expect(decode_file_uris("file:///tmp/raw-音.wav\n")->front() == "/tmp/raw-音.wav", "Preserve UTF-8 local file bytes");
        expect(decode_file_uris("file:///tmp/%ab.dat")->front().native() == std::string("/tmp/\xab.dat"), "Accept lowercase hexadecimal escapes without changing path bytes");
        expect(decode_file_uris("file:///tmp/a\nfile:///tmp/a")->size() == 2, "Preserve path order and repeated entries");
        for (const std::string text : {"", "# only comment\n", "http:///tmp/a", "file://remote/tmp/a", "file://localhost", "file://", "file:///tmp/a?query", "file:///tmp/a#fragment", "file:///tmp/%", "file:///tmp/%0", "file:///tmp/%gg", "file:///tmp/%0g", "file:///tmp/%/0", "file:///tmp/%:0", "file:///tmp/%00", "file:///tmp/%0A", "file:///tmp/%7F", "file:///tmp/\tbad", "file:///tmp/a\nhttp:///tmp/b"})
            expect(!decode_file_uris(text), "Reject invalid, remote, ambiguous or partially invalid URI selections");
        // Both selections are otherwise valid URI lists: an invalid scheme
        // would reject even if the byte-limit guard were accidentally removed.
        const std::string valid_uri = "file:///tmp/limit\n";
        const std::string maximum_bytes = "#" + std::string(65536 - valid_uri.size() - 2, 'x') + "\n" + valid_uri;
        expect(decode_file_uris(maximum_bytes)->front() == "/tmp/limit", "Accept exactly the selection byte limit");
        expect(!decode_file_uris("#" + maximum_bytes), "Reject an otherwise valid selection beyond the byte limit");
        std::string maximum;
        for (int i = 0; i < 256; ++i) maximum += "file:///tmp/" + std::to_string(i) + "\n";
        expect(decode_file_uris(maximum)->size() == 256, "Accept maximum bounded file count");
        expect(!decode_file_uris(maximum + "file:///tmp/overflow"), "Reject excessive file count atomically");
        expect(!decode_file_uris("file:///tmp/" + std::string(4097, 'a')), "Reject paths beyond the bounded path length");
        expect(decode_file_uris("file:///" + std::string(4095, 'a'))->front().native().size() == 4096, "Accept maximum bounded decoded path length");
        std::cout << "PASS: " << checks << " local file URI decoding checks\n";
    } catch (const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; }
}
