"""Fast historical REAPER evidence replay; never a fresh live-DAW claim.

The current-product native/host suites remain separate. This checks actual
retained DAW artifacts without launching REAPER or trusting its PASS summaries.
"""
import base64
import copy
import csv
import hashlib
import io
import json
from pathlib import Path
import re
import struct
import tarfile
import unittest
from unittest.mock import patch
import wave


ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "docs/migrations/evidence/trigger-clap-daw-acceptance-2026-11-02"
PLUGIN_SHA256 = "fd24495233b94466a72de31ec302fd0089f9a4293077766ee533cda77aa552d8"
PATTERNS = (
    ((-16384, 8192), (8192, -4096), (-8192, -16384), (16384, 4096)),
    ((8192, -16384), (4096, 16384), (-12288, 8192), (-8192, 4096)),
)
SELECTED_SHA256 = (
    "4a4a5d51cfadb3339a0a5d87c44342ccf4370baab3abba3f2e4aa5bc2110b7d6",
    "5cee6d9b444f94bbbd94b42cc29b0a0f5393e43e103155b58ac5f96a662670b2",
)


class TriggerClapDawEvidenceTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        # Read regular members only; no archive extraction or host filesystem use.
        with tarfile.open(PACKET / "raw-evidence.tar.gz") as archive:
            cls.members = {m.name: archive.extractfile(m).read()
                           for m in archive if m.isfile() and
                           (m.name.startswith("final/") or m.name == "host-config/reaper.ini")}

    def json_member(self, name):
        return json.loads(self.members["final/" + name])

    def rows(self, name):
        return list(csv.DictReader(io.StringIO(self.members["final/" + name].decode()),
                                   delimiter="\t"))

    def check_render(self, data, slots):
        with wave.open(io.BytesIO(data), "rb") as wav:
            self.assertEqual((wav.getframerate(), wav.getnchannels(), wav.getnframes(),
                              wav.getsampwidth()), (48000, 2, 48000, 3))
            raw = wav.readframes(48000)
        actual = [int.from_bytes(raw[i:i + 3], "little", signed=True)
                  for i in range(0, len(raw), 3)]
        expected = [0] * 96000
        for onset, gains in ((12000, (0.5, 1.5)), (36000, (0.25, 0.75))):
            for frame in range(256):
                # PCM16 stereo mean, physical gain, duplicated PCM24 stereo.
                value = sum(sum(PATTERNS[slot][frame % 4]) * 128 * gains[slot]
                            for slot in slots)
                offset = (onset + frame) * 2
                expected[offset:offset + 2] = [value, value]
        self.assertEqual(len(actual), len(expected))
        for index, (observed, wanted) in enumerate(zip(actual, expected)):
            self.assertEqual(observed, wanted,
                             f"signed PCM/timeline at frame {index // 2}, channel {index % 2}")

    def check_restoration(self, slot, restored):
        sealed = self.members[f"final/sealed-project-state-{slot}.bin"]
        self.assertEqual(restored, sealed, "fresh state differs from saved project")
        self.assertEqual(sealed[:8], b"DTRGSTAT")
        version, metadata_size, payload_size = struct.unpack_from("<IIQ", sealed, 8)
        self.assertEqual(version, 3)
        self.assertEqual(24 + metadata_size + payload_size, len(sealed))
        metadata = json.loads(sealed[24:24 + metadata_size])
        self.assertEqual(metadata["product"], "org.dandrum.trigger")
        parameters = metadata["parameters"]
        self.assertEqual(len(parameters), 23)
        self.assertEqual(len({p["id"] for p in parameters}), 23)
        self.assertEqual(parameters, self.json_member(f"fresh-parameters-{slot}.json"))
        self.assertEqual(next(p["value"] for p in parameters if p["id"] == "drums.level"),
                         (0.5, 1.5)[slot])
        offset = 24 + metadata_size
        self.assertEqual(len(metadata["resources"]), 2)
        selected = [r for r in metadata["resources"] if r["path"].startswith("selected/")]
        self.assertEqual([(r["path"], r["sha256"]) for r in selected],
                         [(f"selected/{SELECTED_SHA256[slot]}.wav", SELECTED_SHA256[slot])],
                         "selected resource identity must occur exactly once")
        for resource in metadata["resources"]:
            content = sealed[offset:offset + resource["encoded_bytes"]]
            self.assertEqual(hashlib.sha256(content).hexdigest(), resource["sha256"])
            offset += resource["encoded_bytes"]
            if resource["path"].startswith("selected/"):
                self.assertEqual(resource["sha256"], SELECTED_SHA256[slot])
                self.assertIn(resource["path"], metadata["document"])
                with wave.open(io.BytesIO(content), "rb") as wav:
                    self.assertEqual((wav.getframerate(), wav.getnchannels(), wav.getnframes(),
                                      wav.getsampwidth()), (48000, 2, 256, 2))
                    self.assertEqual(struct.unpack("<512h", wav.readframes(256)),
                                     tuple(value for _ in range(64)
                                           for pair in PATTERNS[slot] for value in pair))
        self.assertEqual(offset, len(sealed))

    def check_automation(self, lifecycle, gestures):
        # Require nonempty observations before and after each actual transition.
        before = [r for r in lifecycle if 0.1 < float(r["position"]) < 0.4]
        after = [r for r in lifecycle if float(r["position"]) > 0.8]
        self.assertTrue(before and after, "missing timed automation observations")
        self.assertTrue(all((float(r["A"]), float(r["B"])) == (0.5, 1.5)
                            and r["A_reopened"] == "false" for r in before))
        self.assertTrue(all((float(r["A"]), float(r["B"])) == (0.25, 0.75)
                            for r in after))
        self.assertTrue(any(r["A_reopened"] == "true" for r in after))
        held = [r for r in gestures if 0.7 < float(r["elapsed"]) < 1.0]
        released = [r for r in gestures if float(r["elapsed"]) > 1.5]
        self.assertTrue(held and released, "missing held/released native gesture observations")
        self.assertTrue(all((float(r["A"]), float(r["B"]), r["pressed"], r["released"])
                            == (0.25, 1.21, "true", "false") for r in held),
                        "native gesture did not hold the independent value")
        self.assertTrue(all((float(r["A"]), float(r["B"]), r["released"])
                            == (0.25, 0.75, "true") for r in released),
                        "released gesture did not return to automation")

    def test_actual_reaper_renders_match_independent_signed_pcm_and_timing(self):
        for name, slots in (("both-initial", (0, 1)), ("both-fresh", (0, 1)),
                            ("a-fresh", (0,)), ("b-fresh", (1,))):
            with self.subTest(render=name):
                self.check_render(self.members[f"final/renders/{name}.wav"], slots)

    def test_fresh_daw_instances_restore_complete_owned_content_and_physical_values(self):
        project = self.members["final/fresh-restoration-input.rpp"].decode()
        chunks = re.findall(r"<STATE\s*\n(.*?)\n\s*>", project, re.S)
        self.assertEqual(len(chunks), 2)
        for slot, chunk in enumerate(chunks):
            self.assertEqual(base64.b64decode(chunk),
                             self.members[f"final/sealed-project-state-{slot}.bin"])
            self.check_restoration(slot, base64.b64decode(
                self.members[f"final/fresh-restored-{slot}.b64"]))
        self.assertEqual(self.members["final/instance-0-selected.b64"],
                         self.members["final/instance-a-after-b.b64"],
                         "B import changed A's complete state")

    def test_real_host_provenance_and_timed_native_automation_remain_auditable(self):
        provenance = self.json_member("host-provenance.json")
        fresh = self.json_member("fresh-state-verification.json")
        manifest = json.loads((PACKET / "installed-manifest.json").read_text())
        self.assertEqual(next(f["sha256"] for f in manifest["files"]
                              if f["path"] == "lib/clap/DandrumTrigger.clap"), PLUGIN_SHA256)
        self.assertEqual(provenance["version"], "7.84/linux-x86_64")
        self.assertEqual(provenance["plugin_sha256"], PLUGIN_SHA256)
        self.assertEqual(provenance["ime_configuration"], "linux_ime=1")
        self.assertIn(b"linux_ime= 1", self.members["host-config/reaper.ini"])
        self.assertEqual(fresh["plugin_sha256"], PLUGIN_SHA256)
        self.assertEqual(fresh["old_pid"], provenance["pid"])
        self.assertNotEqual(fresh["fresh_pid"], fresh["old_pid"])
        self.assertEqual(fresh["old_exit"], 0)
        self.assertTrue(fresh["original_selected_files_absent"])
        self.assertTrue(fresh["instrumentation_absent"])
        self.assertEqual(self.json_member("fresh-process-exit.json"),
                         {"pid": fresh["fresh_pid"], "exit_status": 0})
        self.check_automation(self.rows("live-editor-lifecycle.tsv"),
                              self.rows("native-gesture-automation.tsv"))

    def test_replay_rejects_signed_state_and_automation_faults(self):
        data = bytearray(self.members["final/renders/both-fresh.wav"])
        # Locate the WAV data chunk; the file has metadata before its PCM.
        offset = 12
        while offset + 8 <= len(data):
            if data[offset:offset + 4] == b"data":
                break
            size = int.from_bytes(data[offset + 4:offset + 8], "little")
            offset += 8 + size + size % 2
        else:
            self.fail("missing WAV data chunk for signed-fault calibration")
        offset += 8 + 12000 * 6
        value = int.from_bytes(data[offset:offset + 3], "little", signed=True)
        self.assertNotEqual(value, 0)
        data[offset:offset + 3] = (-value).to_bytes(3, "little", signed=True)
        with self.assertRaisesRegex(AssertionError, "signed PCM/timeline"):
            self.check_render(data, (0, 1))
        state = bytearray(base64.b64decode(self.members["final/fresh-restored-0.b64"]))
        state[-1] ^= 1
        with self.assertRaisesRegex(AssertionError, "fresh state differs"):
            self.check_restoration(0, state)
        lifecycle = self.rows("live-editor-lifecycle.tsv")
        gestures = copy.deepcopy(self.rows("native-gesture-automation.tsv"))
        for row in gestures:
            if 0.7 < float(row["elapsed"]) < 1.0:
                row["B"] = "0.75"
        with self.assertRaisesRegex(AssertionError, "native gesture did not hold"):
            self.check_automation(lifecycle, gestures)

    def test_replay_requires_selected_content_even_when_saved_and_fresh_states_agree(self):
        sealed_path = "final/sealed-project-state-0.bin"
        renamed = self.members[sealed_path].replace(b"selected/", b"packaged/")
        self.assertNotEqual(renamed, self.members[sealed_path])
        with patch.object(self, "members", {**self.members, sealed_path: renamed}):
            with self.assertRaisesRegex(AssertionError, "selected resource identity"):
                self.check_restoration(0, renamed)


if __name__ == "__main__":
    unittest.main()
