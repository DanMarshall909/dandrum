#include "Tb303WebUi.h"

#include <iostream>
#include <string_view>

int main()
{
    const std::string_view html { Tb303WebUi::indexHtml };

    if (html.find ("const specs=") != std::string_view::npos)
    {
        std::cerr << "web UI still declares a hard-coded parameter surface\n";
        return 1;
    }

    if (html.find ("renderParameters") == std::string_view::npos
        || html.find ("parameterValuesChanged") == std::string_view::npos)
    {
        std::cerr << "web UI does not render and update the processor's active parameters\n";
        return 1;
    }

    if (html.find ("native('noteOn')") == std::string_view::npos
        || html.find ("native('noteOff')") == std::string_view::npos)
    {
        std::cerr << "web UI keyboard does not call the native note bridge\n";
        return 1;
    }

    return 0;
}
