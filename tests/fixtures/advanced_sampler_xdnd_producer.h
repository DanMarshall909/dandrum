#pragma once
#include <X11/Xatom.h>
#include <X11/Xlib.h>
#include <array>
#include <chrono>
#include <stdexcept>
#include <string>

// Reused real wire producer from AdvancedSamplerNativeFileDropTest. This test
// source owns XdndSelection and uses XSendEvent/XChangeProperty, never an app
// callback or a synthetic Slint drop. exactTitle selects the composed window.
namespace dandrum::test {
using Clock = std::chrono::steady_clock;
class NativeXdndProducer {
public:
    enum class End { Drop, DuplicateDrop, Leave, DelayedDrop };
    explicit NativeXdndProducer(std::string exactTitle, std::string uri, bool extendedTypes = false, End end = End::Drop, bool unsupportedMime = false, bool wrongFormat = false, bool ignoreSelection = false)
        : title_(std::move(exactTitle)), uri_(std::move(uri)), display_(XOpenDisplay(nullptr)), extendedTypes_(extendedTypes), end_(end), wrongFormat_(wrongFormat), ignoreSelection_(ignoreSelection) {
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
    ~NativeXdndProducer() { XDestroyWindow(display_, source_); XCloseDisplay(display_); }
    void poll() {
        if (!started_) discover();
        while (XPending(display_)) {
            XEvent event{}; XNextEvent(display_, &event);
            if (event.type == SelectionRequest) {
                if (ignoreSelection_) continue;
                const auto& request = event.xselectionrequest;
                XEvent reply{}; reply.xselection.type = SelectionNotify;
                reply.xselection.display = display_; reply.xselection.requestor = request.requestor;
                reply.xselection.selection = request.selection; reply.xselection.target = request.target;
                reply.xselection.time = request.time;
                reply.xselection.property = None;
                if (request.selection == selection_ && request.target == mime_) {
                    const auto property = request.property == None ? request.target : request.property;
                    if (wrongFormat_) {
                        const unsigned long malformed = 0x1234;
                        XChangeProperty(display_, request.requestor, property, mime_, 32, PropModeReplace,
                                        reinterpret_cast<const unsigned char*>(&malformed), 1);
                    } else XChangeProperty(display_, request.requestor, property, mime_, 8, PropModeReplace,
                                           reinterpret_cast<const unsigned char*>(uri_.data()), static_cast<int>(uri_.size()));
                    reply.xselection.property = property; served_ = true; servedAt_ = Clock::now();
                }
                XSendEvent(display_, request.requestor, False, NoEventMask, &reply); XFlush(display_);
            } else if (event.type == ClientMessage && event.xclient.message_type == atom("XdndStatus")) {
                statusSeen_ = true; statusAt_ = Clock::now();
                accepted_ = event.xclient.data.l[0] == static_cast<long>(target_) && (event.xclient.data.l[1] & 1);
            } else if (event.type == ClientMessage && event.xclient.message_type == atom("XdndFinished")) {
                finished_ = true; success_ = event.xclient.data.l[0] == static_cast<long>(target_) && (event.xclient.data.l[1] & 1);
            }
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
    bool targetHasProxy() { discover(false); return target_ && destination_ != target_; }
    bool hovering() const { return served_ && !sentDrop_; }
    bool finished() const { return finished_ || (end_ == End::Leave && sentDrop_ && Clock::now() - endedAt_ > std::chrono::milliseconds(200)); }
    bool exchanged() const { return statusSeen_ && (!accepted_ || served_ || ignoreSelection_) && finished(); }
    bool delivered() const { return accepted_ && served_ && finished_ && success_; }
private:
    Atom atom(const char* name) { return XInternAtom(display_, name, False); }
    unsigned long property(Window window, const char* name) {
        Atom type{}; int format{}; unsigned long count{}, remaining{}; unsigned char* bytes{};
        XGetWindowProperty(display_, window, atom(name), 0, 1, False, XA_WINDOW,
                           &type, &format, &count, &remaining, &bytes);
        const auto value = type == XA_WINDOW && format == 32 && count == 1 ? reinterpret_cast<unsigned long*>(bytes)[0] : 0;
        if (bytes) XFree(bytes);
        return value;
    }
    void discover(bool sendMessages = true) {
        Window root{}, parent{}, *children{}; unsigned int count{};
        if (!XQueryTree(display_, DefaultRootWindow(display_), &root, &parent, &children, &count)) return;
        for (unsigned int i = 0; i < count; ++i) {
            char* name{};
            if (XFetchName(display_, children[i], &name) && name) {
                const bool matches = std::string(name) == title_; XFree(name);
                if (matches) { target_ = children[i]; break; }
            }
        }
        if (children) XFree(children);
        if (!target_) return;
        destination_ = target_;
        if (auto proxy = property(target_, "XdndProxy"); proxy && property(proxy, "XdndProxy") == proxy) destination_ = proxy;
        if (!sendMessages) return;
        started_ = true;
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
    std::string title_, uri_;
    bool started_{};
    Display* display_{}; Window source_{}, target_{}, destination_{}; Atom selection_{}, mime_{};
    bool accepted_{}, served_{}, sentDrop_{}, finished_{}, success_{}, statusSeen_{};
    bool extendedTypes_{};
    End end_;
    bool wrongFormat_{}, ignoreSelection_{};
    Clock::time_point servedAt_{}, endedAt_{}, statusAt_{};
};
}
