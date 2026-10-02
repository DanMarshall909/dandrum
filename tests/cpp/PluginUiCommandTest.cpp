#include "InstrumentUiCommands.h"

#include <functional>
#include <iostream>
#include <stdexcept>

namespace
{
using Status = InstrumentUiCommandStatus;

void require (bool condition, const char* message)
{
    if (! condition)
        throw std::runtime_error (message);
}

struct RecordingHost final : InstrumentUiCommandHost
{
    std::uint32_t uiCommandGeneration() const noexcept override { return generation; }
    Status applyUiParameter (std::uint32_t, const std::string&, float, bool) override
    {
        return Status::accepted;
    }
    InstrumentUiGestureAdmission beginUiGesture (std::uint32_t, const std::string& id) override
    {
        if (id != "fixture.level")
            return { Status::unknownControl };
        ++begins;
        const auto callback = onBegin;
        if (callback)
            callback();
        return { Status::accepted, 7 };
    }
    void endUiGesture (std::size_t slot) override
    {
        ++ends;
        lastSlot = slot;
        const auto callback = onEnd;
        if (callback)
            callback();
    }

    std::uint32_t generation = 17;
    int begins = 0;
    int ends = 0;
    std::size_t lastSlot = 0;
    std::function<void()> onBegin;
    std::function<void()> onEnd;
};
}

int main()
{
    try
    {
        {
            RecordingHost host;
            InstrumentUiCommandService service (host);
            const auto session = service.createSession();
            require (service.beginGesture ({ 16, "fixture.level", session }).status == Status::staleGeneration
                         && service.beginGesture ({ 17, "fixture.level", 0 }).status == Status::gestureActive
                         && service.beginGesture ({ 17, "missing", session }).status == Status::unknownControl
                         && host.begins == 0 && host.ends == 0,
                     "invalid begin notified the host");
            require (service.beginGesture ({ 17, "fixture.level", session }).status == Status::accepted
                         && service.beginGesture ({ 17, "fixture.level", session }).status == Status::gestureActive
                         && service.endGesture ({ 17, "missing", session }).status == Status::unknownControl,
                     "active gesture lost its control ownership");
            service.closeSession (session);
            service.closeSession (session);
            require (host.begins == 1 && host.ends == 1 && host.lastSlot == 7,
                     "ordinary repeated session closure did not end exactly once");
            const auto next = service.createSession();
            service.beginGesture ({ 17, "fixture.level", next });
            require (service.endGesture ({ 16, "fixture.level", next }).status == Status::staleGeneration
                         && host.begins == 2 && host.ends == 2,
                     "stale end retained its host gesture");
        }
        RecordingHost host;
        InstrumentUiCommandService service (host);
        const auto session = service.createSession();
        host.onBegin = [&]
        {
            require (service.beginGesture ({ 17, "fixture.level", session }).status == Status::gestureActive,
                     "a pending host begin admitted a second begin");
            service.closeSession (session);
        };
        require (service.beginGesture ({ 17, "fixture.level", session }).status == Status::accepted,
                 "host-reentrant begin was not admitted");
        require (host.begins == 1 && host.ends == 1 && host.lastSlot == 7,
                 "closing during host begin left an unowned gesture");
        require (service.endGesture ({ 17, "fixture.level", session }).status == Status::noGesture,
                 "closed pending begin retained a gesture");
        host.onBegin = {};
        const auto reopened = service.createSession();
        require (service.beginGesture ({ 17, "fixture.level", reopened }).status == Status::accepted
                     && service.endGesture ({ 17, "fixture.level", reopened }).status == Status::accepted
                     && host.begins == 2 && host.ends == 2 && host.lastSlot == 7,
                 "a new editor session could not gesture after closure during begin");
        for (const bool closeInsteadOfEnd : { false, true })
        {
            RecordingHost endingHost;
            InstrumentUiCommandService endingService (endingHost);
            const auto endingSession = endingService.createSession();
            endingService.beginGesture ({ 17, "fixture.level", endingSession });
            endingHost.onEnd = [&]
            {
                require (endingHost.ends == 1, "closing during host end notified the host twice");
                endingService.closeSession (endingSession);
            };
            if (closeInsteadOfEnd)
                endingService.closeSession (endingSession);
            else
                require (endingService.endGesture ({ 17, "fixture.level", endingSession }).status
                             == Status::accepted, "reentrant host end was not admitted");
            require (endingHost.begins == 1 && endingHost.ends == 1 && endingHost.lastSlot == 7
                         && endingService.endGesture ({ 17, "fixture.level", endingSession }).status
                             == Status::noGesture,
                     "closure during end did not retire the original gesture exactly once");
        }
    }
    catch (const std::exception& error)
    {
        std::cerr << error.what() << '\n';
        return 1;
    }
    return 0;
}
