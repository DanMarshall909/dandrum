#include "X11FileDrop.h"
#include "FileUris.h"
#include <xcb/xcb.h>
#include <xcb/xcbext.h>
#include <unistd.h>
#include <algorithm>
#include <array>
#include <chrono>
#include <cstdlib>
#include <cstring>
#include <list>
#include <optional>

namespace dandrum::sampler {
struct X11FileDrop::State {
    using Clock = std::chrono::steady_clock;
    struct Atoms {
        xcb_atom_t name, utf8, pid, proxy, aware, enter, position, leave, drop,
            status, finished, selection, uri, types, copy, data;
    } atoms{};
    struct Job {
        unsigned sequence;
        bool discovery;
        std::function<void(void*, xcb_generic_error_t*)> complete;
    };
    struct Candidate { xcb_window_t window; bool mapped{}, owned{}, available{}; std::string title; };
    xcb_connection_t* connection{};
    xcb_window_t root{}, target{}, proxy{}, source{};
    std::string title;
    Received received;
    std::list<Job> jobs;
    std::vector<std::shared_ptr<Candidate>> candidates;
    unsigned discoveryPending{}, visited{}, epoch{};
    bool scanning{}, supported{}, converted{}, ready{}, dropped{}, accepted{};
    std::optional<Files> paths;
    Clock::time_point nextScan{}, deadline{};

    State(std::string exactTitle, Received callback) : title(std::move(exactTitle)), received(std::move(callback)) {
        // No DISPLAY means Wayland or a display-free process. That path keeps
        // using the normal Slint platform and needs no X11 adapter.
        const auto display = std::getenv("DISPLAY");
        if (!display || !*display || title.empty()) return;
        int screenNumber = 0;
        connection = xcb_connect(nullptr, &screenNumber);
        if (xcb_connection_has_error(connection)) { xcb_disconnect(connection); connection = nullptr; return; }
        auto screen = xcb_setup_roots_iterator(xcb_get_setup(connection));
        for (int i = 0; i < screenNumber && screen.rem; ++i) xcb_screen_next(&screen);
        if (!screen.rem) { xcb_disconnect(connection); connection = nullptr; return; }
        root = screen.data->root;
        const std::array names{"_NET_WM_NAME", "UTF8_STRING", "_NET_WM_PID", "XdndProxy", "XdndAware", "XdndEnter", "XdndPosition", "XdndLeave", "XdndDrop", "XdndStatus", "XdndFinished", "XdndSelection", "text/uri-list", "XdndTypeList", "XdndActionCopy", "_DANDRUM_XDND_DATA"};
        std::array<xcb_intern_atom_cookie_t, names.size()> cookies{};
        for (std::size_t i = 0; i < names.size(); ++i) cookies[i] = xcb_intern_atom(connection, 0, std::strlen(names[i]), names[i]);
        std::array<xcb_atom_t, names.size()> values{};
        // One-time setup before the UI timer starts. All timer-time replies
        // below use xcb_poll_for_reply, which never waits for the X server.
        for (std::size_t i = 0; i < values.size(); ++i) {
            auto reply = xcb_intern_atom_reply(connection, cookies[i], nullptr);
            if (reply) { values[i] = reply->atom; std::free(reply); }
        }
        if (std::ranges::any_of(values, [](auto atom) { return atom == XCB_ATOM_NONE; })) {
            xcb_disconnect(connection); connection = nullptr; return;
        }
        atoms = {values[0], values[1], values[2], values[3], values[4], values[5], values[6], values[7], values[8], values[9], values[10], values[11], values[12], values[13], values[14], values[15]};
    }
    ~State() {
        if (!connection) return;
        if (target && proxy && !xcb_connection_has_error(connection)) {
            // A final round trip only during destruction (after UI shutdown),
            // so a replacement installed by somebody else is never removed.
            const auto cookie = xcb_get_property(connection, 0, target, atoms.proxy, XCB_ATOM_WINDOW, 0, 1);
            auto reply = xcb_get_property_reply(connection, cookie, nullptr);
            if (reply) {
                if (word(reply, XCB_ATOM_WINDOW) == proxy) xcb_delete_property(connection, target, atoms.proxy);
                std::free(reply);
            }
        }
        if (proxy) xcb_destroy_window(connection, proxy);
        // Complete our deletes before disconnecting: otherwise another client
        // can observe a dangling proxy between our flush and server processing.
        if (!xcb_connection_has_error(connection)) {
            auto fence = xcb_get_input_focus_reply(connection, xcb_get_input_focus(connection), nullptr);
            std::free(fence);
        }
        xcb_disconnect(connection);
    }
    static uint32_t word(const xcb_get_property_reply_t* reply, xcb_atom_t type) {
        if (!reply || reply->type != type || reply->format != 32 || reply->bytes_after || xcb_get_property_value_length(reply) != 4) return 0;
        uint32_t value{}; std::memcpy(&value, xcb_get_property_value(reply), sizeof value); return value;
    }
    void queue(unsigned sequence, bool discovery, std::function<void(void*, xcb_generic_error_t*)> complete) {
        if (discovery) ++discoveryPending;
        jobs.push_back({sequence, discovery, std::move(complete)});
    }
    void property(xcb_window_t window, xcb_atom_t name, xcb_atom_t type, unsigned length, bool discovery,
                  std::function<void(const xcb_get_property_reply_t*)> complete) {
        const auto cookie = xcb_get_property(connection, 0, window, name, type, 0, length);
        queue(cookie.sequence, discovery, [complete = std::move(complete)](void* reply, auto error) {
            complete(error ? nullptr : static_cast<xcb_get_property_reply_t*>(reply));
        });
    }
    void inspect(xcb_window_t window) {
        auto candidate = std::make_shared<Candidate>(); candidate->window = window;
        candidates.push_back(candidate);
        property(window, atoms.pid, XCB_ATOM_CARDINAL, 1, true, [candidate](auto reply) {
            candidate->owned = word(reply, XCB_ATOM_CARDINAL) == static_cast<uint32_t>(getpid());
        });
        property(window, atoms.name, atoms.utf8, 256, true, [this, candidate](auto reply) {
            if (!reply || reply->bytes_after || reply->format != 8 || reply->type != atoms.utf8) return;
            candidate->title.assign(static_cast<const char*>(xcb_get_property_value(reply)), xcb_get_property_value_length(reply));
        });
        const auto attributes = xcb_get_window_attributes(connection, window);
        queue(attributes.sequence, true, [candidate](void* reply, auto error) {
            candidate->mapped = !error && reply && static_cast<xcb_get_window_attributes_reply_t*>(reply)->map_state == XCB_MAP_STATE_VIEWABLE;
        });
        property(window, atoms.proxy, XCB_GET_PROPERTY_TYPE_ANY, 1, true, [candidate](auto reply) {
            // Even a malformed/foreign property is somebody else's ownership.
            candidate->available = reply && reply->type == XCB_ATOM_NONE;
        });
    }
    void tree(xcb_window_t parent, unsigned depth) {
        const auto cookie = xcb_query_tree(connection, parent);
        queue(cookie.sequence, true, [this, depth](void* value, auto error) {
            if (error || !value) return;
            auto reply = static_cast<xcb_query_tree_reply_t*>(value);
            const auto children = xcb_query_tree_children(reply);
            for (int i = 0; i < xcb_query_tree_children_length(reply) && visited < 256; ++i) {
                if (children[i] == proxy) continue;
                ++visited; inspect(children[i]);
                if (depth < 8) tree(children[i], depth + 1);
            }
        });
    }
    void attach(xcb_window_t window) {
        // Recheck immediately before installing. Never replace an existing
        // foreign proxy, including malformed legacy properties.
        property(window, atoms.proxy, XCB_GET_PROPERTY_TYPE_ANY, 1, false, [this, window](auto reply) {
            if (target || !reply || reply->type != XCB_ATOM_NONE) return;
            target = window; proxy = xcb_generate_id(connection);
            xcb_create_window(connection, 0, proxy, root, 0, 0, 1, 1, 0, XCB_WINDOW_CLASS_INPUT_ONLY, XCB_COPY_FROM_PARENT, 0, nullptr);
            const uint32_t version = 5;
            xcb_change_property(connection, XCB_PROP_MODE_REPLACE, proxy, atoms.proxy, XCB_ATOM_WINDOW, 32, 1, &proxy);
            xcb_change_property(connection, XCB_PROP_MODE_REPLACE, proxy, atoms.aware, XCB_ATOM_ATOM, 32, 1, &version);
            xcb_change_property(connection, XCB_PROP_MODE_REPLACE, target, atoms.proxy, XCB_ATOM_WINDOW, 32, 1, &proxy);
            const uint32_t mask = XCB_EVENT_MASK_STRUCTURE_NOTIFY;
            xcb_change_window_attributes(connection, target, XCB_CW_EVENT_MASK, &mask);
        });
    }
    void reset() {
        ++epoch; source = 0; supported = converted = ready = dropped = accepted = pendingPosition = false; paths.reset();
        if (proxy) xcb_delete_property(connection, proxy, atoms.data);
    }
    void send(xcb_atom_t type, std::array<uint32_t, 5> data) {
        if (!source) return;
        xcb_client_message_event_t event{};
        event.response_type = XCB_CLIENT_MESSAGE; event.window = source; event.type = type; event.format = 32;
        std::copy(data.begin(), data.end(), event.data.data32);
        xcb_send_event(connection, 0, source, XCB_EVENT_MASK_NO_EVENT, reinterpret_cast<const char*>(&event));
    }
    void finish() {
        if (!dropped || !ready) return;
        const bool success = paths && received;
        send(atoms.finished, {target, success ? 1u : 0u, success ? atoms.copy : uint32_t{XCB_ATOM_NONE}, 0, 0});
        auto files = std::move(paths); reset();
        if (success) received(std::move(*files));
    }
    void position(uint32_t time, uint32_t action) {
        accepted = supported && (action == atoms.copy || action == XCB_ATOM_NONE);
        send(atoms.status, {target, accepted ? 3u : 2u, 0, 0, accepted ? atoms.copy : uint32_t{XCB_ATOM_NONE}});
        if (accepted && !converted) {
            converted = true;
            selectionTime = time;
            deadline = Clock::now() + std::chrono::seconds(5);
            xcb_convert_selection(connection, proxy, atoms.selection, atoms.uri, atoms.data, time);
        }
    }
    void client(const xcb_client_message_event_t& event) {
        if (event.format != 32 || event.window != target) return;
        const auto* data = event.data.data32;
        if (event.type == atoms.enter) {
            reset(); source = data[0];
            const auto version = data[1] >> 24;
            if (!source || version < 3 || version > 5) return;
            if (!(data[1] & 1)) supported = data[2] == atoms.uri || data[3] == atoms.uri || data[4] == atoms.uri;
            else {
                const auto session = epoch;
                property(source, atoms.types, XCB_ATOM_ATOM, 64, false, [this, session](auto reply) {
                    if (session != epoch || !reply || reply->format != 32 || reply->type != XCB_ATOM_ATOM || reply->bytes_after) return;
                    const auto values = static_cast<const xcb_atom_t*>(xcb_get_property_value(reply));
                    supported = std::find(values, values + xcb_get_property_value_length(reply) / 4, atoms.uri) != values + xcb_get_property_value_length(reply) / 4;
                    if (pendingPosition) { position(pendingTime, pendingAction); pendingPosition = false; }
                });
            }
        } else if (source && data[0] == source) {
            if (event.type == atoms.position) {
                pendingTime = data[3]; pendingAction = data[4];
                // Inline types are ready immediately; extended types complete
                // asynchronously, then respond to the latest pointer position.
                if (supported) position(data[3], data[4]);
                else { pendingPosition = true; position(data[3], data[4]); }
            } else if (event.type == atoms.leave) reset();
            else if (event.type == atoms.drop) {
                dropped = true;
                if (!accepted) { ready = true; paths.reset(); }
                finish();
            }
        }
    }
    bool pendingPosition{};
    uint32_t pendingTime{}, pendingAction{}, selectionTime{};
    void selection(const xcb_selection_notify_event_t& event) {
        if (!source || event.requestor != proxy || event.selection != atoms.selection || event.target != atoms.uri
            || event.time != selectionTime || !converted || ready) return;
        if (event.property != atoms.data) { ready = true; paths.reset(); finish(); return; }
        const auto session = epoch;
        property(proxy, atoms.data, XCB_GET_PROPERTY_TYPE_ANY, 16384, false, [this, session](auto reply) {
            if (session != epoch) return;
            if (reply && reply->type == atoms.uri && reply->format == 8 && !reply->bytes_after)
                paths = decode_file_uris({static_cast<const char*>(xcb_get_property_value(reply)), static_cast<std::size_t>(xcb_get_property_value_length(reply))});
            else paths.reset();
            ready = true; finish();
        });
    }
    void poll() {
        if (!connection || xcb_connection_has_error(connection)) return;
        auto remaining = std::min<std::size_t>(jobs.size(), 128);
        for (auto iterator = jobs.begin(); iterator != jobs.end() && remaining--; ) {
            void* reply{}; xcb_generic_error_t* error{};
            if (!xcb_poll_for_reply(connection, iterator->sequence, &reply, &error)) { ++iterator; continue; }
            auto job = std::move(*iterator); iterator = jobs.erase(iterator);
            if (job.discovery) --discoveryPending;
            job.complete(reply, error); std::free(reply); std::free(error);
        }
        for (unsigned i = 0; i < 128; ++i) {
            auto event = xcb_poll_for_event(connection); if (!event) break;
            const auto kind = event->response_type & 127;
            if (kind == XCB_CLIENT_MESSAGE) client(*reinterpret_cast<xcb_client_message_event_t*>(event));
            else if (kind == XCB_SELECTION_NOTIFY) selection(*reinterpret_cast<xcb_selection_notify_event_t*>(event));
            else if (kind == XCB_DESTROY_NOTIFY && reinterpret_cast<xcb_destroy_notify_event_t*>(event)->window == target) {
                reset(); target = 0; xcb_destroy_window(connection, proxy); proxy = 0;
            }
            std::free(event);
        }
        // Bound an unanswered selection request, not the user's hover time.
        // A ready selection remains valid until Drop or Leave.
        if (source && converted && !ready && Clock::now() >= deadline) { ready = true; paths.reset(); finish(); }
        if (!target && !scanning && Clock::now() >= nextScan) {
            scanning = true; visited = 0; candidates.clear(); tree(root, 0); nextScan = Clock::now() + std::chrono::milliseconds(250);
        }
        if (scanning && !discoveryPending) {
            scanning = false;
            std::vector<xcb_window_t> matches;
            for (const auto& candidate : candidates)
                if (candidate->mapped && candidate->owned && candidate->available && candidate->title == title) matches.push_back(candidate->window);
            if (matches.size() == 1) attach(matches.front());
        }
        xcb_flush(connection);
    }
};
X11FileDrop::X11FileDrop(std::string title, Received received) : state_(std::make_unique<State>(std::move(title), std::move(received))) {}
X11FileDrop::~X11FileDrop() = default;
void X11FileDrop::poll() { state_->poll(); }
bool X11FileDrop::active() const { return state_->connection && !xcb_connection_has_error(state_->connection) && state_->target != XCB_WINDOW_NONE; }
}
