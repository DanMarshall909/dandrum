#include "AdvancedSamplerNativeFileDrop.h"
#include <slint-platform.h>
#include <array>
#include <chrono>
#include <filesystem>
#include <fstream>
#include <iostream>
#include <optional>
#include <stdexcept>
#include <vector>
#include <X11/Xatom.h>
#include <X11/Xlib.h>
#include <unistd.h>
#ifdef DANDRUM_HAVE_X11_FILE_DROP
#include "ui/slint/platform/WindowFileDrop.h"
#endif

namespace {
constexpr const char* title = "Dandrum native XDND delivery test";
using Clock = std::chrono::steady_clock;

// A separate real X11 source owns the URI selection and performs the XDND wire
// exchange. It never calls a Slint callback or synthesizes a Slint drop event.
class Producer {
public:
    enum class End { Drop, DuplicateDrop, Leave, DelayedDrop, StaleSelection };
    explicit Producer(std::string uri, bool extendedTypes = false, End end = End::Drop, bool unsupportedMime = false, bool wrongFormat = false, bool ignoreSelection = false)
        : uri_(std::move(uri)), display_(XOpenDisplay(nullptr)), extendedTypes_(extendedTypes), end_(end), wrongFormat_(wrongFormat), ignoreSelection_(ignoreSelection) {
        if (!display_) throw std::runtime_error("Missing private X11 display");
        source_ = XCreateSimpleWindow(display_, DefaultRootWindow(display_), 0, 0, 1, 1, 0, 0, 0);
        selection_ = atom("XdndSelection");
        mime_ = atom(unsupportedMime ? "text/plain" : "text/uri-list");
        XSetSelectionOwner(display_, selection_, source_, CurrentTime);
        if (extendedTypes_) {
            const std::array<Atom, 4> types{atom("text/plain"), atom("application/json"), atom("text/html"), mime_};
            XChangeProperty(display_, source_, atom("XdndTypeList"), XA_ATOM, 32, PropModeReplace,
                            reinterpret_cast<const unsigned char*>(types.data()), types.size());
        }
    }
    ~Producer() { XDestroyWindow(display_, source_); XCloseDisplay(display_); }
    void poll() {
        if (!target_) discover();
        while (XPending(display_)) {
            XEvent event{}; XNextEvent(display_, &event);
            if (event.type == SelectionRequest) {
                if (ignoreSelection_) continue;
                const auto& request = event.xselectionrequest;
                if (end_ == End::StaleSelection) {
                    deferred_ = request; deferredAt_ = Clock::now();
                    respond(request, true);
                } else respond(request, false);
            } else if (event.type == ClientMessage && event.xclient.message_type == atom("XdndStatus")) {
                statusSeen_ = true; statusAt_ = Clock::now();
                accepted_ = event.xclient.data.l[0] == static_cast<long>(target_) && (event.xclient.data.l[1] & 1);
            } else if (event.type == ClientMessage && event.xclient.message_type == atom("XdndFinished")) {
                finished_ = true; success_ = event.xclient.data.l[0] == static_cast<long>(target_) && (event.xclient.data.l[1] & 1);
            }
        }
        if (deferred_ && Clock::now() - deferredAt_ > std::chrono::milliseconds(100)) {
            respond(*deferred_, false); deferred_.reset();
        }
        const auto hover = std::chrono::milliseconds(end_ == End::DelayedDrop ? 5200 : 100);
        if (!sentDrop_ && ((accepted_ && served_ && Clock::now() - servedAt_ > hover) ||
                          (accepted_ && ignoreSelection_ && Clock::now() - statusAt_ > std::chrono::milliseconds(100)) ||
                          (statusSeen_ && !accepted_ && Clock::now() - statusAt_ > std::chrono::milliseconds(100)))) {
            if (end_ == End::Leave) send("XdndLeave", {static_cast<long>(source_), 0, 0, 0, 0});
            else {
                send("XdndDrop", {static_cast<long>(source_), 0, CurrentTime, 0, 0});
                if (end_ == End::DuplicateDrop) send("XdndDrop", {static_cast<long>(source_), 0, CurrentTime, 0, 0});
            }
            sentDrop_ = true; endedAt_ = Clock::now();
        }
    }
    bool finished() const { return finished_ || (end_ == End::Leave && sentDrop_ && Clock::now() - endedAt_ > std::chrono::milliseconds(200)); }
    bool exchanged() const { return statusSeen_ && (!accepted_ || served_ || ignoreSelection_) && finished(); }
    bool delivered() const { return accepted_ && served_ && finished_ && success_; }
    bool selectionReady() const { return accepted_ && served_ && !sentDrop_; }
private:
    void respond(const XSelectionRequestEvent& request, bool stale) {
        XEvent reply{}; reply.xselection.type = SelectionNotify;
        reply.xselection.display = display_; reply.xselection.requestor = request.requestor;
        reply.xselection.selection = request.selection; reply.xselection.target = request.target;
        // A cancelled drag can finish its conversion after the next request.
        // Its old timestamp and paths must not satisfy the current request.
        reply.xselection.time = stale ? request.time - 1 : request.time;
        reply.xselection.property = None;
        if (request.selection == selection_ && request.target == mime_) {
            const auto property = request.property == None ? request.target : request.property;
            const std::string selection = stale ? "file:///tmp/cancelled-drag.wav" : uri_;
            if (wrongFormat_) {
                const unsigned long malformed = 0x1234;
                XChangeProperty(display_, request.requestor, property, mime_, 32, PropModeReplace,
                                reinterpret_cast<const unsigned char*>(&malformed), 1);
            } else XChangeProperty(display_, request.requestor, property, mime_, 8, PropModeReplace,
                                   reinterpret_cast<const unsigned char*>(selection.data()), static_cast<int>(selection.size()));
            reply.xselection.property = property;
            if (!stale) { served_ = true; servedAt_ = Clock::now(); }
        }
        XSendEvent(display_, request.requestor, False, NoEventMask, &reply); XFlush(display_);
    }
    Atom atom(const char* name) { return XInternAtom(display_, name, False); }
    unsigned long property(Window window, const char* name) {
        Atom type{}; int format{}; unsigned long count{}, remaining{}; unsigned char* bytes{};
        XGetWindowProperty(display_, window, atom(name), 0, 1, False, XA_WINDOW,
                           &type, &format, &count, &remaining, &bytes);
        const auto value = type == XA_WINDOW && format == 32 && count == 1 ? reinterpret_cast<unsigned long*>(bytes)[0] : 0;
        if (bytes) XFree(bytes);
        return value;
    }
    void discover() {
        Window root{}, parent{}, *children{}; unsigned int count{};
        if (!XQueryTree(display_, DefaultRootWindow(display_), &root, &parent, &children, &count)) return;
        for (unsigned int i = 0; i < count; ++i) {
            XWindowAttributes attributes{};
            if (!XGetWindowAttributes(display_, children[i], &attributes) || attributes.map_state != IsViewable) continue;
            char* name{};
            if (XFetchName(display_, children[i], &name) && name) {
                const bool matches = std::string(name) == title; XFree(name);
                if (matches) { target_ = children[i]; break; }
            }
        }
        if (children) XFree(children);
        if (!target_) return;
        destination_ = target_;
        if (auto proxy = property(target_, "XdndProxy"); proxy && property(proxy, "XdndProxy") == proxy) destination_ = proxy;
        send("XdndEnter", {static_cast<long>(source_), (5L << 24) | (extendedTypes_ ? 1 : 0), extendedTypes_ ? 0 : static_cast<long>(mime_), 0, 0});
        int x{}, y{}; Window child{};
        XTranslateCoordinates(display_, target_, DefaultRootWindow(display_), 40, 40, &x, &y, &child);
        send("XdndPosition", {static_cast<long>(source_), 0, (x << 16) | (y & 0xffff), CurrentTime, static_cast<long>(atom("XdndActionCopy"))});
    }
    void send(const char* name, std::array<long, 5> data) {
        XEvent event{}; event.xclient.type = ClientMessage; event.xclient.display = display_;
        event.xclient.window = target_; event.xclient.message_type = atom(name); event.xclient.format = 32;
        for (int i = 0; i < 5; ++i) event.xclient.data.l[i] = data[i];
        XSendEvent(display_, destination_, False, NoEventMask, &event); XFlush(display_);
    }
    std::string uri_;
    Display* display_{}; Window source_{}, target_{}, destination_{}; Atom selection_{}, mime_{};
    bool accepted_{}, served_{}, sentDrop_{}, finished_{}, success_{}, statusSeen_{};
    bool extendedTypes_{};
    End end_;
    bool wrongFormat_{}, ignoreSelection_{};
    std::optional<XSelectionRequestEvent> deferred_;
    Clock::time_point servedAt_{}, endedAt_{}, statusAt_{}, deferredAt_{};
};
}

int main(int argc, char** argv) {
    try {
        const auto directory = argc > 1 ? std::filesystem::absolute(argv[1]) : std::filesystem::temp_directory_path();
        const auto path = directory / ("xdnd native 音 input-" + std::to_string(getpid()) + ".dat");
        const auto second = directory / "xdnd-other+input.dat";
        { std::ofstream file(path); file << "real native URI selection"; }
        { std::ofstream file(second); file << "second native selection path"; }
        auto encode = [](const std::filesystem::path& path) {
            std::string uri = "file://";
            constexpr char digits[] = "0123456789ABCDEF";
            for (const unsigned char c : path.native()) {
                if (c == ' ' || c >= 128 || c == '%' || c == '#' || c == '?') {
                    uri += '%'; uri += digits[c >> 4]; uri += digits[c & 15];
                } else uri += c;
            }
            return uri;
        };
        auto window = dandrum_ui::NativeFileDropTest::create();
        int receipts = 0; std::vector<std::filesystem::path> received;
        window->on_received([&](slint::DataTransfer transfer) {
            ++receipts; const auto files = transfer.file_paths(); if (files) received.assign(files->begin(), files->end());
        });
#ifdef DANDRUM_HAVE_X11_FILE_DROP
        auto adapter = dandrum::slint_ui::bind_window_file_drop(window, title,
            [](const auto& target, slint::DataTransfer transfer) { target->invoke_received(std::move(transfer)); });
#endif
        struct Case { std::string selection; std::vector<std::filesystem::path> expected; bool extendedTypes = false; Producer::End end = Producer::End::Drop; bool unsupportedMime = false; bool wrongFormat = false; bool ignoreSelection = false; };
        const std::vector<Case> cases{
            {encode(path) + "\r\n", {path}},
#ifdef DANDRUM_HAVE_X11_FILE_DROP
            {"# real desktop multi-file selection\r\n" + encode(path) + "\r\nfile://localhost" + second.native() + "\n", {path, second}},
            {"file://foreign-host" + second.native(), {}},
            {"file:///tmp/invalid%00.dat", {}},
            {encode(path) + "\nhttp:///tmp/second-invalid", {}},
            {"# no usable paths\n", {}},
            {"file:///tmp/" + std::string(65537, 'x'), {}},
            {encode(path), {path}, true},
            {encode(path), {path}, false, Producer::End::DuplicateDrop},
            {encode(path), {}, false, Producer::End::Leave},
            {encode(path), {}, false, Producer::End::Drop, true},
            {encode(path), {}, false, Producer::End::Drop, false, true},
            {encode(path), {}, false, Producer::End::Drop, false, false, true},
            {encode(second), {second}},
            {encode(path), {path}, false, Producer::End::StaleSelection},
            {encode(path), {path}, false, Producer::End::DelayedDrop},
#endif
        };
        std::size_t current = 0; int successful = 0;
        auto makeSource = [&](const Case& test) { return std::make_unique<Producer>(test.selection, test.extendedTypes, test.end, test.unsupportedMime, test.wrongFormat, test.ignoreSelection); };
        auto source = makeSource(cases.front());
        slint::Timer timer; auto began = Clock::now();
        int ticks = 0;
        std::string failure;
        timer.start(slint::TimerMode::Repeated, std::chrono::milliseconds(10), [&] {
            ++ticks;
#ifdef DANDRUM_HAVE_X11_FILE_DROP
            
            if (adapter->active()) source->poll();
#else
            source->poll();
#endif
            if (source->finished()) {
                const auto& expected = cases[current].expected;
                if (!source->exchanged() || source->delivered() != !expected.empty())
                    failure = "Real XDND exchange must report success only for a valid local URI selection";
                else if (expected.empty() ? receipts != successful : receipts != successful + 1 || received != expected)
                    failure = "Desktop file drop must deliver its exact ordered paths once, and reject invalid selections without receipt";
                else if (cases[current].ignoreSelection && ticks < 50)
                    failure = "UI timer must remain responsive while an unanswered native selection times out";
                if (!expected.empty()) {
                    ++successful;
                    std::cout << "Exact receipt " << successful << ':';
                    for (const auto& path : received) std::cout << ' ' << path;
                    std::cout << '\n';
                }
                ++current;
                if (!failure.empty() || current == cases.size()) slint::quit_event_loop();
                else { source = makeSource(cases[current]); began = Clock::now(); ticks = 0; }
            } else if (Clock::now() - began > std::chrono::seconds(cases[current].end == Producer::End::DelayedDrop || cases[current].ignoreSelection ? 10 : 5)) {
                failure = "XDND setup did not complete a real accepted URI selection exchange";
                slint::quit_event_loop();
            }
        });
        window->run();
        if (!failure.empty()) throw std::runtime_error(failure);
        std::cout << "PASS: " << current << " real XDND exchanges; application receipts=" << receipts
                  << "; exact spaces/Unicode/multi-path delivery and atomic local URI rejection\n";
#ifdef DANDRUM_HAVE_X11_FILE_DROP
        timer.stop(); window->hide(); adapter.reset(); source.reset();
        std::optional abandoned{dandrum_ui::NativeFileDropTest::create()};
        int abandonedReceipts = 0;
        (*abandoned)->on_received([&](slint::DataTransfer) { ++abandonedReceipts; });
        const auto weak = slint::ComponentWeakHandle<dandrum_ui::NativeFileDropTest>(*abandoned);
        // Keep the native adapter alive after hiding and destroying the component.
        // This lets a real in-flight XDND exchange finish and exercises weak.lock(),
        // rather than succeeding because destruction removed the native target.
        slint::Window nativeLifetime((*abandoned)->window().window_handle());
        auto orphanAdapter = dandrum::slint_ui::bind_window_file_drop(*abandoned, title,
            [](const auto& target, slint::DataTransfer transfer) { target->invoke_received(std::move(transfer)); });
        (*abandoned)->show();
        auto orphanSource = std::make_unique<Producer>(encode(path));
        bool expired = false; std::string orphanFailure;
        const auto orphanBegan = Clock::now();
        slint::Timer orphanTimer;
        orphanTimer.start(slint::TimerMode::Repeated, std::chrono::milliseconds(10), [&] {
            if (orphanAdapter->active()) orphanSource->poll();
            if (!expired && orphanSource->selectionReady()) {
                (*abandoned)->hide(); abandoned.reset(); expired = !weak.lock().has_value();
                if (!expired) { orphanFailure = "The native lifetime fixture must actually destroy the receiving component"; slint::quit_event_loop(); }
            }
            if (orphanSource->finished()) {
                if (!expired || !orphanSource->exchanged() || !orphanSource->delivered() || abandonedReceipts != 0)
                    orphanFailure = "A real completed native selection must suppress delivery when its receiving component expired";
                slint::quit_event_loop();
            } else if (Clock::now() - orphanBegan > std::chrono::seconds(5)) {
                orphanFailure = "The expired-window native selection did not complete"; slint::quit_event_loop();
            }
        });
        slint::run_event_loop(slint::EventLoopMode::RunUntilQuit);
        if (orphanFailure.empty() && (!expired || !orphanSource->finished() || !orphanSource->exchanged()
                || !orphanSource->delivered() || abandonedReceipts != 0))
            orphanFailure = "The expired-component assertion requires a completed native exchange, not an early event-loop exit";
        orphanTimer.stop(); nativeLifetime.hide(); orphanAdapter.reset();
        if (!orphanFailure.empty()) throw std::runtime_error(orphanFailure);
        std::cout << "PASS: expired component receives no callback after a real native selection completes\n";
#endif
    } catch (const std::exception& error) {
        std::cerr << "FAIL: " << error.what() << '\n'; return 1;
    }
}
