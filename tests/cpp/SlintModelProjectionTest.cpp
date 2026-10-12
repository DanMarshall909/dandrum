#include "ui/slint/host/ModelProjection.h"
#include <algorithm>
#include <iostream>
#include <map>
#include <stdexcept>
#define CHECK(x) do { ++checks; if(!(x)) throw std::runtime_error(#x); } while(false)
struct ProjectionRow {
 slint::SharedString id;
 int value=0;
 std::shared_ptr<slint::Model<int>> children;
 bool operator==(const ProjectionRow&) const = default;
};
struct CountingProjection : slint::VectorModel<ProjectionRow> {
 using slint::VectorModel<ProjectionRow>::VectorModel;
 mutable std::size_t reads=0;
 mutable bool invalidRead=false;
 std::optional<ProjectionRow> row_data(std::size_t i) const override {
  ++reads;invalidRead|=i>=row_count();return slint::VectorModel<ProjectionRow>::row_data(i);
 }
};
struct ThrowingProjection : slint::VectorModel<int> {
 using slint::VectorModel<int>::VectorModel;
 std::optional<int> row_data(std::size_t) const override { throw std::runtime_error("consumer read failure"); }
};
void stableProjection(int& checks) {
 namespace binding = dandrum::slint_ui::binding;
 struct Record { std::string id; int value; };
 std::vector<Record> records;
 std::vector<ProjectionRow> initial;
 std::map<std::string,std::shared_ptr<slint::Model<int>>> children;
 for(int i=0;i<128;++i) {
  auto id="row"+std::to_string(i);
  auto child=std::make_shared<slint::VectorModel<int>>(std::vector<int>{i});
  records.push_back({id,i});
  initial.push_back({binding::str(id),i,child});children.emplace(id,child);
 }
 auto counted=std::make_shared<CountingProjection>(std::move(initial));
 std::shared_ptr<slint::Model<ProjectionRow>> model=counted;
 auto fill=[](ProjectionRow& row,const auto& record) {
  row.id=binding::str(record.id);row.value=record.value;
  if(!row.children)row.children=std::make_shared<slint::VectorModel<int>>(std::vector<int>{row.value});
 };
 auto verify=[&](bool moved=false) {
  auto oldCount=model->row_count();counted->reads=0;counted->invalidRead=false;
  auto result=binding::project(model,records,fill);
  CHECK(result==model);
  CHECK(counted->reads<=(moved?3:2)*std::max(oldCount,records.size()));
  CHECK(!counted->invalidRead);CHECK(result->row_count()==records.size());
  for(std::size_t i=0;i<records.size();++i) {
   auto row=result->row_data(i);CHECK(row.has_value());CHECK(row->id==binding::str(records[i].id));
   CHECK(row->value==records[i].value);
   auto existing=children.find(records[i].id);
   if(existing!=children.end())CHECK(row->children==existing->second);
   else {
    for(const auto& [id,child]:children)CHECK(row->children!=child);
    CHECK(row->children->row_count()==1);CHECK(row->children->row_data(0)==row->value);
    children.emplace(records[i].id,row->children);
   }
  }
 };
 verify();
 records.front().value=999;verify();
 records.insert(records.begin(),{"inserted",42});verify(true);
 records.erase(records.begin()+1);verify(true);
 std::reverse(records.begin(),records.end());verify(true);
 records.clear();verify();
 records.push_back({"fresh",7});verify();
}

int main() {
 int checks=0;
 try {
  stableProjection(checks);
  namespace binding = dandrum::slint_ui::binding;
  std::shared_ptr<slint::Model<int>> plain;
  struct PlainRecord { int value; };
  const std::vector<PlainRecord> records{{7},{9}};
  auto projected=binding::project(plain,records,[](int& row,const PlainRecord& record){row=record.value;});
  CHECK(projected->row_count()==2); CHECK(projected->row_data(0)==7); CHECK(projected->row_data(1)==9);
  plain=projected;
  CHECK(binding::project(plain,records,[](int& row,const PlainRecord& record){row=record.value;})==plain);
  auto text=binding::strings(std::shared_ptr<slint::Model<slint::SharedString>>{},"first;;second;");
  CHECK(text->row_count()==2); CHECK(text->row_data(0)=="first"); CHECK(text->row_data(1)=="second");
  std::shared_ptr<slint::Model<slint::SharedString>> oldText=text;
  CHECK(binding::strings(oldText,"first;second")==oldText);
  CHECK(binding::strings(oldText,"")->row_count()==0);
  // A host-supplied model may fail while reading. Preserve that failure rather
  // than publishing partially projected data or silently replacing the model.
  auto throwing=std::make_shared<ThrowingProjection>(std::vector<int>{3});
  std::shared_ptr<slint::Model<int>> failed=throwing;
  bool propagated=false;
  try { binding::synchronize(failed,std::vector<int>{7}); }
  catch(const std::runtime_error& error) { propagated=std::string_view(error.what())=="consumer read failure"; }
  CHECK(propagated);
  CHECK(throwing->slint::VectorModel<int>::row_data(0)==3);
  std::cout << "PASS: " << checks << " independent model projection checks\n";
  return 0;
 } catch(const std::exception& error) { std::cerr << "FAIL: " << error.what() << '\n'; return 1; }
}
