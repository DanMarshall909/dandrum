#pragma once
#include <slint.h>
#include <memory>
#include <optional>
#include <sstream>
#include <string>
#include <string_view>
#include <unordered_map>
#include <utility>
#include <vector>

// UI-thread projection preserves stable model/child identities and emits only
// changed rows. Record containers supply string IDs; fill owns domain mapping.
namespace dandrum::slint_ui::binding {
// SharedString requires UTF-8 text without embedded NUL bytes.
inline slint::SharedString str(std::string_view s){return slint::SharedString(std::string(s).c_str());}
template<class P> using Row=typename decltype(std::declval<P>()->row_data(0))::value_type;
template<class P> auto synchronize(const P& previous,std::vector<Row<P>> rows){
 auto model=std::dynamic_pointer_cast<slint::VectorModel<Row<P>>>(previous);
 if(!model)model=std::make_shared<slint::VectorModel<Row<P>>>();
 if(model->row_count()!=rows.size())model->set_vector(std::move(rows));
 else for(std::size_t i=0;i<rows.size();++i)if(model->row_data(i)!=std::optional<Row<P>>(rows[i]))model->set_row_data(i,rows[i]);
 return model;
}
template<class P,class Records,class F> auto project(const P& previous,const Records& records,F fill){
 std::vector<Row<P>> rows;rows.reserve(records.size());
 std::optional<std::unordered_map<std::string,Row<P>>> previousById;
 for(auto& record:records){
  Row<P> row{};
  // Preserve child models by record identity, including when rows are reordered.
  // The usual unchanged-order path needs only one lookup per record.
  if constexpr(requires { row.id; })if(previous){
   const auto id=str(record.id);
   auto old=rows.size()<previous->row_count()?previous->row_data(rows.size()):std::optional<Row<P>>{};
   if(old&&old->id==id)row=*old;
   else {
    // Index once when rows move, so insertion/deletion cannot trigger quadratic scans.
    if(!previousById){
     previousById.emplace();previousById->reserve(previous->row_count());
     for(std::size_t i=0;i<previous->row_count();++i)if(auto candidate=previous->row_data(i)){
      auto key=std::string(std::string_view(candidate->id));
      previousById->emplace(std::move(key),std::move(*candidate));
     }
    }
    if(auto found=previousById->find(record.id);found!=previousById->end())row=found->second;
   }
  }
  fill(row,record);rows.push_back(std::move(row));
 }
 return synchronize(previous,std::move(rows));
}
template<class P> auto strings(const P& previous,std::string_view delimited){std::vector<slint::SharedString> out;std::istringstream in{std::string(delimited)};std::string s;while(std::getline(in,s,';'))if(!s.empty())out.push_back(str(s));return synchronize(previous,std::move(out));}
}
