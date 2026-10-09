#include "ui/advanced-sampler/render/WaveDisplay.h"
#include <iostream>
#include <limits>
#include <stdexcept>

namespace {
int checks = 0;
void expect(bool ok, const char* message) { ++checks; if (!ok) throw std::runtime_error(message); }
struct Bucket { float minimum{}, maximum{}; };
struct SparseModel : slint::Model<Bucket> {
    mutable std::size_t reads = 0;
    std::size_t row_count() const override { return 8192; }
    std::optional<Bucket> row_data(std::size_t row) const override { ++reads; return row == 4096 ? std::nullopt : std::optional(Bucket{-.5f, .25f}); }
};
slint::Color color(std::uint32_t rgb) { return slint::Color::from_argb_uint8(255, rgb >> 16, rgb >> 8, rgb); }
}
int main() { try {
    using namespace dandrum::advanced_sampler;
    constexpr std::array<std::uint32_t,5> palette{0x130f0c,0x41362c,0xb0662f,0xe08a4e,0xf2e6d3};
    auto model=std::make_shared<slint::VectorModel<Bucket>>(std::vector<Bucket>{{-1,.25f},{-.5f,.25f},{0,0},{-.25f,.1f}});
    WaveDisplayCache cache;
    const auto request=[&](const auto& data, bool reversed=false, float start=0, float end=1, slint::Color accent=color(0xe08a4e), std::string_view profile="kick"){
        return cache.image_for_model(data,reversed,start,end,color(palette[0]),color(palette[1]),color(palette[2]),accent,color(palette[4]),profile);
    };
    const auto first=request(model); const auto first_pixels=first.to_rgba8();
    expect(first_pixels&&first_pixels->width()==4&&first_pixels->height()==64,"Native spectral Image has actual bounded RGBA pixels");
    expect(first_pixels->cbegin()[63*4].r==242&&first_pixels->cbegin()[63*4].g==230&&first_pixels->cbegin()[63*4].b==211,"Native adapter uses signed min/max amplitudes and the exact known reference low-frequency pixel");
    const auto same=request(model).to_rgba8();
    expect(same&&same->cbegin()==first_pixels->cbegin(),"Repeated native callback reuses its cached pixel buffer");
    const auto equal_model=std::make_shared<slint::VectorModel<Bucket>>(std::vector<Bucket>{{-1,.25f},{-.5f,.25f},{0,0},{-.25f,.1f}});
    expect(request(equal_model).to_rgba8()->cbegin()==first_pixels->cbegin(),"Projection replacement with unchanged amplitudes reuses the image");
    expect(request(model,false,0,1,color(0xe08a4e),"unknown").to_rgba8()->cbegin()==first_pixels->cbegin(),"Unknown profile shares the effective reference kick cache entry");
    const std::array<float,4> original_amplitudes{1,.5f,0,.25f};
    for(std::size_t role=0;role<palette.size();++role){
        const auto baseline=cache.image(original_amplitudes,"kick",false,0,1,palette).to_rgba8();
        auto edited_palette=palette;edited_palette[role]=0x00ff00;
        const auto edited=cache.image(original_amplitudes,"kick",false,0,1,edited_palette).to_rgba8();
        bool different=false;
        for(unsigned i=0;i<4*64;++i)different=different||baseline->cbegin()[i].r!=edited->cbegin()[i].r||baseline->cbegin()[i].g!=edited->cbegin()[i].g||baseline->cbegin()[i].b!=edited->cbegin()[i].b;
        expect(baseline->cbegin()!=edited->cbegin()&&different,"Each of the five visible palette roles invalidates the cache and changes actual native pixels");
    }
    expect(request(model,true).to_rgba8()->cbegin()!=first_pixels->cbegin(),"Reversal invalidates the native cached image");
    const auto restored=request(model).to_rgba8();
    expect(request(model,false,.25f,.5f).to_rgba8()->cbegin()!=restored->cbegin(),"Region edits invalidate the native cached image");
    const auto region=request(model,false,.25f,.5f).to_rgba8();
    expect(request(model,false,.25f,.5f,color(0x00ff00)).to_rgba8()->cbegin()!=region->cbegin(),"Visible accent changes invalidate cached spectral colors");
    const auto themed=request(model,false,.25f,.5f,color(0x00ff00)).to_rgba8();
    expect(request(model,false,.25f,.5f,color(0x00ff00),"snare").to_rgba8()->cbegin()!=themed->cbegin(),"Profile changes invalidate the native cached image");
    const auto before=request(model).to_rgba8();
    model->set_row_data(0,Bucket{-.25f,.1f});
    const auto changed=request(model).to_rgba8();
    expect(changed->cbegin()!=before->cbegin()&&changed->cbegin()[63*4].r!=before->cbegin()[63*4].r,"Prepared amplitude updates change actual cached pixels");
    expect(first_pixels->cbegin()[63*4].r==242,"Previously supplied native image remains immutable after cache updates");
    auto huge=std::make_shared<SparseModel>();
    const auto bounded=request(huge).to_rgba8();
    expect(bounded&&bounded->width()==4096&&bounded->height()==64&&huge->reads==4096,"Native model reads and output memory are both bounded");
    std::shared_ptr<slint::Model<Bucket>> missing;
    expect(request(missing).size().width==0,"Null waveform clears the native image");
    auto empty=std::make_shared<slint::VectorModel<Bucket>>();
    expect(request(empty).size().width==0,"Empty waveform supplies no fabricated native image");
    std::cout<<"PASS: "<<checks<<" native spectral image/cache checks\n";
}catch(const std::exception& error){std::cerr<<"FAIL: "<<error.what()<<'\n';return 1;}}
