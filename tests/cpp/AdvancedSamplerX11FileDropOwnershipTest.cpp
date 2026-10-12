#include "ui/advanced-sampler/platform/X11FileDrop.h"
#include <X11/Xatom.h>
#include <X11/Xlib.h>
#include <unistd.h>
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <iostream>
#include <memory>
#include <stdexcept>
#include <thread>

namespace {
using Drop = dandrum::sampler::X11FileDrop;
constexpr const char* title = "Dandrum XDND ownership · fixture";
class Desktop {
public:
    Display* display = XOpenDisplay(nullptr);
    Desktop() { if (!display) throw std::runtime_error("Missing private X11 display"); }
    ~Desktop() { XCloseDisplay(display); }
    Atom atom(const char* name) { return XInternAtom(display, name, False); }
    Window window(const char* name, unsigned long pid, Window parent = 0) {
        auto window = XCreateSimpleWindow(display, parent ? parent : DefaultRootWindow(display), 0, 0, 100, 100, 0, 0, 0);
        XStoreName(display, window, name);
        XChangeProperty(display, window, atom("_NET_WM_NAME"), atom("UTF8_STRING"), 8, PropModeReplace,
                        reinterpret_cast<const unsigned char*>(name), std::strlen(name));
        XChangeProperty(display, window, atom("_NET_WM_PID"), XA_CARDINAL, 32, PropModeReplace,
                        reinterpret_cast<const unsigned char*>(&pid), 1);
        XMapWindow(display, window); XSync(display, False); return window;
    }
    void property(Window window, const char* name, unsigned long value) {
        XChangeProperty(display, window, atom(name), XA_WINDOW, 32, PropModeReplace,
                        reinterpret_cast<const unsigned char*>(&value), 1); XSync(display, False);
    }
    unsigned long proxy(Window window) {
        Atom type{}; int format{}; unsigned long count{}, remaining{}; unsigned char* bytes{};
        XGetWindowProperty(display, window, atom("XdndProxy"), 0, 1, False, AnyPropertyType, &type, &format, &count, &remaining, &bytes);
        const auto value = type == XA_WINDOW && format == 32 && count == 1 ? reinterpret_cast<unsigned long*>(bytes)[0] : 0;
        if (bytes) XFree(bytes);
        return value;
    }
    void destroy(Window window) { XDestroyWindow(display, window); XSync(display, False); }
};
void settle(Drop& adapter, bool active) {
    const auto until = std::chrono::steady_clock::now() + std::chrono::milliseconds(active ? 1000 : 350);
    do { adapter.poll(); std::this_thread::sleep_for(std::chrono::milliseconds(2)); }
    while (std::chrono::steady_clock::now() < until && (!active || !adapter.active()));
}
}
int main() {
    int checks = 0;
    auto expect = [&](bool condition, const char* message) { ++checks; if (!condition) throw std::runtime_error(message); };
    try {
        Desktop desktop;
        auto foreign = desktop.window(title, getpid() + 100000);
        auto wrongTitle = desktop.window("Other same-process editor", getpid());
        {
            Drop adapter(title, [](auto) {}); settle(adapter, false);
            expect(!adapter.active() && !desktop.proxy(foreign) && !desktop.proxy(wrongTitle), "Exact title and current PID both required before attaching");
        }
        desktop.destroy(foreign); desktop.destroy(wrongTitle);
        auto hidden = desktop.window(title, getpid());
        XUnmapWindow(desktop.display, hidden); XSync(desktop.display, False);
        {
            Drop adapter(title, [](auto) {}); settle(adapter, false);
            expect(!adapter.active() && !desktop.proxy(hidden), "Unmapped editor windows are not desktop drop targets");
        }
        desktop.destroy(hidden);
        auto noPid = desktop.window(title, getpid());
        XDeleteProperty(desktop.display, noPid, desktop.atom("_NET_WM_PID")); XSync(desktop.display, False);
        {
            Drop adapter(title, [](auto) {}); settle(adapter, false);
            expect(!adapter.active() && !desktop.proxy(noPid), "Missing process ownership cannot attach by title alone");
        }
        desktop.destroy(noPid);
        auto target = desktop.window(title, getpid());
        auto otherProxy = desktop.window("Foreign proxy", getpid());
        desktop.property(target, "XdndProxy", otherProxy);
        {
            Drop adapter(title, [](auto) {}); settle(adapter, false);
            expect(!adapter.active() && desktop.proxy(target) == otherProxy, "Leave an existing foreign XdndProxy untouched");
        }
        XDeleteProperty(desktop.display, target, desktop.atom("XdndProxy")); XSync(desktop.display, False);
        {
            Drop adapter(title, [](auto) {}); settle(adapter, true);
            const auto proxy = desktop.proxy(target);
            expect(adapter.active() && proxy && desktop.proxy(proxy) == proxy, "Install a standard self-referencing proxy on only the matching window");
        }
        expect(!desktop.proxy(target), "RAII removes our target proxy property after adapter shutdown");
        {
            Drop adapter(title, [](auto) {}); settle(adapter, true);
            desktop.property(target, "XdndProxy", otherProxy);
        }
        expect(desktop.proxy(target) == otherProxy, "RAII preserves a replacement proxy owned by somebody else");
        desktop.destroy(target); desktop.destroy(otherProxy);
        auto first = desktop.window(title, getpid()); auto second = desktop.window(title, getpid());
        {
            Drop adapter(title, [](auto) {}); settle(adapter, false);
            expect(!adapter.active() && !desktop.proxy(first) && !desktop.proxy(second), "An ambiguous same-PID title match cannot attach to an arbitrary window");
        }
        desktop.destroy(first); desktop.destroy(second);
        auto frame = desktop.window("Window manager frame", getpid() + 100000);
        auto child = desktop.window(title, getpid(), frame);
        {
            Drop adapter(title, [](auto) {}); settle(adapter, true);
            expect(adapter.active() && desktop.proxy(child) && !desktop.proxy(frame), "Discover reparented editor without installing properties on its window-manager frame");
            desktop.destroy(child); settle(adapter, false);
            expect(!adapter.active(), "Destroyed target detaches without touching unrelated windows");
            child = desktop.window(title, getpid(), frame); settle(adapter, true);
            expect(adapter.active() && desktop.proxy(child), "Adapter can discover a recreated editor window");
        }
        desktop.destroy(frame);
        const std::string display = std::getenv("DISPLAY");
        unsetenv("DISPLAY");
        { Drop adapter(title, [](auto) {}); adapter.poll(); expect(!adapter.active(), "No X11 adapter is activated on a display-free or Wayland-only process"); }
        setenv("DISPLAY", display.c_str(), 1);
        { Drop adapter("", [](auto) {}); adapter.poll(); expect(!adapter.active(), "An empty title cannot take over another window"); }
        std::cout << "PASS: " << checks << " native X11 ownership and lifecycle checks\n";
    } catch (const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; }
}
