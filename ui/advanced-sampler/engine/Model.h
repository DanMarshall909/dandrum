#pragma once
#include <cstdint>
#include <map>
#include <memory>
#include <optional>
#include <set>
#include <string>
#include <string_view>
#include <vector>
namespace dandrum::sampler {
// Editor data is owned on the UI thread. It is never an audio-engine snapshot.
struct Record {
 std::string id;
 std::map<std::string,std::string> fields;
 std::string text(std::string_view key, std::string fallback={}) const;
 double number(std::string_view key,double fallback=0) const;
};
struct Peak { float minimum{}, maximum{}; };
struct AdapterEvent { std::uint64_t generation{}; std::string kind, detail; };
class EngineAdapter {
public:
 virtual ~EngineAdapter()=default;
 virtual void submit(std::uint64_t generation,std::string_view operation,std::string_view definition)=0;
 virtual void noteOn(int note,int velocity,std::string_view owner)=0;
 virtual void noteOff(int note,std::string_view owner)=0;
 virtual std::string name() const=0;
};
class SilentMockAdapter final: public EngineAdapter {
public:
 void submit(std::uint64_t,std::string_view,std::string_view) override {}
 void noteOn(int,int,std::string_view) override {}
 void noteOff(int,std::string_view) override {}
 std::string name() const override { return "Silent mock adapter · no audio engine connected"; }
};
class Model {
public:
 explicit Model(std::shared_ptr<EngineAdapter> adapter=std::make_shared<SilentMockAdapter>());
 bool command(std::string_view action,std::string_view target={},std::string_view text={},double value=0);
 bool gesture(std::string_view id,int phase,double value);
 bool noteOn(int note,int velocity,std::string_view source);
 void noteOff(int note,std::string_view source);
 void releaseNotes();
 bool pcKey(std::string_view key,bool pressed,bool textInput,bool modified);
 void tick(double seconds);
 bool receive(const AdapterEvent& event);
 bool selectState(std::string_view id);
 static const std::vector<std::string>& stateIds();
 static const std::vector<std::string>& commandIds();
 const std::vector<Record>& records(std::string_view table) const;
 const Record* find(std::string_view table,std::string_view id) const;
 std::optional<Record> parameterInfo(std::string_view id) const;
 std::vector<Record> processorParameters(std::string_view table,std::string_view id) const;
 std::vector<Record> autosamplerParameters() const;
 std::string text(std::string_view id) const;
 double number(std::string_view id) const;
 const std::vector<Peak>& peaks() const { return peaks_; }
 std::vector<int> heldNotes() const;
 std::vector<std::string> selectCandidates(int note,int velocity);
 std::string definition() const;
 bool importDefinition(std::string_view data);
 double effective(std::string_view id,std::string_view field) const;
 std::string outputFor(std::string_view id) const;
 std::size_t undoCount() const { return undo_.size(); }
 std::size_t redoCount() const { return redo_.size(); }
 std::string undoLabel() const;
 std::string redoLabel() const;
 std::uint64_t generation() const { return generation_; }
private:
 using Tables=std::map<std::string,std::vector<Record>>;
 struct Change { std::string table,id; std::optional<Record> before,after; std::size_t index{}; };
 struct Edit { std::string label; std::vector<Change> changes; bool structural{}; };
 struct Gesture { std::string id; Edit edit; };
 struct Job { std::uint64_t generation{};std::string kind,target,path;double elapsed{};bool fail{}; };
 Tables tables_;
 std::shared_ptr<EngineAdapter> adapter_;
 std::vector<Peak> peaks_;
 std::map<std::string,std::vector<Peak>> sourcePeaks_;
 std::vector<Edit> undo_,redo_;
 std::optional<Gesture> gesture_;
 std::vector<Job> jobs_;
 std::map<std::string,int> owners_,pcHeld_;
 std::map<int,int> cycles_;
 std::map<int,std::string> lastSelections_;
 std::vector<Record> clipboard_;
 std::uint64_t generation_{1},serial_{0},eventSequence_{0};
 std::uint32_t random_{0xDAD123};
 std::string lastAnalysis_{"transients"};
 bool applying_{};
 void initialize();
 void loadPreset(std::string_view id);
 void loadGuideFixture(std::string_view state);
 void selectSource(std::string_view source);
 void set(std::string_view id,std::string_view value);
 void set(std::string_view id,double value);
 void event(std::string_view kind,std::string_view label);
 Record make(std::string id,std::initializer_list<std::pair<const std::string,std::string>> fields) const;
 std::string uid(std::string_view prefix);
 void apply(const Change&,bool forward);
 void commit(Edit edit);
 void put(Edit&,std::string_view table,Record record);
 void erase(Edit&,std::string_view table,std::string_view id);
 bool field(Edit&,std::string_view table,std::string_view id,std::string_view key,std::string value);
 void setting(Edit&,std::string_view id,std::string value);
 void beginJob(std::string kind,std::string target={},std::string path={},bool fail=false);
 void finishJob(const Job&);
 bool readWav(std::string_view path,Record& result,std::vector<Peak>& peaks,std::string& error) const;
 bool parameter(std::string_view id,double value,bool record);
 bool gestureValue(std::string_view id,double value,Edit& edit);
 void updateRegion(Edit&,std::string_view id,double value);
 bool tableCommand(std::string_view action,std::string_view target,std::string_view text,double value,Edit& edit);
};
}
