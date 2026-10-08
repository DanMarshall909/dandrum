#include "Catalog.h"
#include "host/ValueCodec.h"

int main() {
    auto window = dandrum_ui::CatalogWindow::create();
    dandrum::slint_ui::bind_value_codec<dandrum_ui::ParsedValue>(
        window->global<dandrum_ui::ValueCodec>());
    window->run();
    return 0;
}
