#pragma once
#include <slint.h>
#include "DesignTokens.h"
#include <string_view>

namespace dandrum::slint_ui {
namespace search_filter_detail {
inline bool contains(const slint::SharedString& text, const slint::SharedString& query)
{
    return std::string_view{text.data(), text.size()}.find(
               std::string_view{query.data(), query.size()}) != std::string_view::npos;
}

template<class Preset>
bool matches(const Preset& preset, const slint::SharedString& query,
             const slint::SharedString& category)
{
    if (!(category == tokens::editor_browser_category_all.data() || (category == tokens::editor_browser_category_favourites.data() && preset.favourite)
          || (category == tokens::editor_browser_category_user.data() && preset.origin == tokens::editor_browser_category_user.data()) || category == preset.category))
        return false;
    return query.empty()
        || contains((preset.name + tokens::editor_strings_space.data() + preset.category
                     + tokens::editor_strings_space.data() + preset.contents).to_lowercase(), query);
}
} // namespace search_filter_detail

// Bind all search operations so native catalogs are not limited by the preview
// fallback. Slint's Unicode lowercase conversion matches the row visibility rule.
template<class SearchFilterGlobal>
void bind_search_filter(const SearchFilterGlobal& filter)
{
    filter.on_contains(search_filter_detail::contains);
    filter.on_count([](const auto& presets, slint::SharedString query, slint::SharedString category) {
        if (!presets) return 0;
        presets->track_any_change();
        query = query.to_lowercase();
        int count = 0;
        for (std::size_t i = 0; i < presets->row_count(); ++i) {
            const auto row = presets->row_data(i);
            if (row && search_filter_detail::matches(*row, query, category)) ++count;
        }
        return count;
    });
    filter.on_first([](const auto& presets, slint::SharedString query, slint::SharedString category) {
        if (!presets) return slint::SharedString{};
        presets->track_any_change();
        query = query.to_lowercase();
        for (std::size_t i = 0; i < presets->row_count(); ++i) {
            const auto row = presets->row_data(i);
            if (row && search_filter_detail::matches(*row, query, category)) return row->id;
        }
        return slint::SharedString{};
    });
}

} // namespace dandrum::slint_ui
