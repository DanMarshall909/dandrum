"""Replay the assertions on the retained Linux VST3 REAPER acceptance capture."""

import argparse
import base64
import hashlib
import io
import json
from pathlib import Path
import re
import struct
import unittest
import wave


ROOT = Path(__file__).resolve().parents[2]
PACKET = ROOT / "docs/migrations/evidence/trigger-vst3-reaper-2026-11-24"
FIXTURES = {
    "A": ("eedc7e98b27f7fb773ec03aaf8fb5139f0ac87bc1f4a1245c1158e03f1e1ffd8",
          (-16384, 8192, -8192, 16384) + (-8192,) * 60),
    "B": ("bcc2072b36f5a6d12525ee5db25e1b586e5347bf6bf89302e8e3676a40769e06",
          (-8192, 16384, -16384, 8192) + (4096,) * 60),
}
TRIGGER_BLOCK = re.compile(
    r'^[ \t]*<VST "VST3i: Dandrum Trigger \(Dandrum\)" .*?\n(.*?)^[ \t]*>',
    re.MULTILINE | re.DOTALL,
)
STATE_MAGIC = b"DTRGSTAT"
FRAMES = 960


def sha256(data):
    return hashlib.sha256(data).hexdigest()


def read_json(path):
    return json.loads(path.read_text())


class TriggerVst3ReaperCaptureTest(unittest.TestCase):
    capture = None

    @classmethod
    def setUpClass(cls):
        if cls.capture is None:
            raise unittest.SkipTest("requires the private REAPER qualification capture")
        cls.receipt = read_json(PACKET / "verification.json")
        cls.private = cls.capture / "private"
        cls.first = cls.private / "first"
        cls.fresh = cls.private / "fresh"

    def test_installed_bundle_and_both_offline_reaper_processes(self):
        bundle = self.private / "DandrumTrigger.vst3"
        files = [
            {"path": path.relative_to(bundle).as_posix(), "sha256": sha256(path.read_bytes()),
             "bytes": path.stat().st_size, "mode": oct(path.stat().st_mode & 0o777)}
            for path in sorted(bundle.rglob("*")) if path.is_file()
        ]
        bundle_digest = sha256(json.dumps(files, sort_keys=True).encode())
        self.assertEqual({"files": len(files), "digest": bundle_digest}, self.receipt["bundle"])
        binary = self.receipt["source"]["binarySha256"]
        self.assertEqual(sha256((bundle / "Contents/x86_64-linux/DandrumTrigger.so").read_bytes()),
                         binary)

        identities = []
        for stage, directory in (("first", self.first), ("fresh", self.fresh)):
            loaded = read_json(self.capture / f"{stage}-loaded-module.json")
            rendered = read_json(self.capture / f"{stage}-after-renders-module.json")
            revision = rendered.pop("reaperRevision")
            self.assertEqual(revision.rstrip("\0"), "7.84")
            self.assertEqual(loaded.pop("reaperRevision", revision), revision)
            self.assertEqual(loaded, rendered)
            recorded = self.receipt["processes"][stage]["loadedBeforeAndAfterRenders"]
            self.assertEqual(loaded, recorded)
            self.assertEqual(loaded["binarySha256"], binary)
            self.assertEqual(loaded["bundleDigest"], bundle_digest)
            self.assertTrue(any(" r-xp " in row and row.endswith(loaded["binaryPath"])
                                for row in loaded["maps"]))
            self.assertTrue(loaded["sourceHidden"])
            self.assertEqual(loaded["networkRoutes"], 0)
            self.assertIn("linux_ime=1", (directory / "reaper.ini").read_text())
            self.assertEqual(read_json(directory / "host-receipt.json")["reaperExit"], 0)
            identities.append((loaded["pid"], loaded["processStartTick"]))
        self.assertNotEqual(identities[0], identities[1], "restore must use a fresh process")

    def read_project_states(self, project, *, undone=False):
        blocks = TRIGGER_BLOCK.findall(project.read_text())
        self.assertEqual(len(blocks), 2, project.name)
        states = {}
        for (label, (selected, expected_pcm)), block in zip(FIXTURES.items(), blocks, strict=True):
            encoded = b"".join(base64.b64decode(line.strip(), validate=True)
                               for line in block.splitlines())
            self.assertEqual(encoded.count(STATE_MAGIC), 1, project.name)
            start = encoded.index(STATE_MAGIC)
            version, metadata_size, payload_size = struct.unpack_from("<IIQ", encoded, start + 8)
            self.assertEqual(version, 3)
            content_start = start + 24 + metadata_size
            end = content_start + payload_size
            metadata = json.loads(encoded[start + 24:content_start])
            self.assertEqual(metadata["product"], "org.dandrum.trigger")
            self.assertEqual(len(metadata["parameters"]), 23)
            self.assertEqual(len({entry["id"] for entry in metadata["parameters"]}), 23)
            payload = encoded[content_start:end]
            self.assertEqual(len(payload), payload_size)
            offset = 0
            selected_paths = []
            for resource in metadata["resources"]:
                limit = offset + resource["encoded_bytes"]
                content = payload[offset:limit]
                self.assertEqual(sha256(content), resource["sha256"], resource["path"])
                offset = limit
                if resource["path"].startswith("selected/"):
                    selected_paths.append(resource["path"])
                    self.assertEqual(resource["sha256"], selected, f"{label} selected clip")
                    with wave.open(io.BytesIO(content), "rb") as audio:
                        self.assertEqual(audio.getparams()[:4], (1, 2, 48000, len(expected_pcm)))
                        self.assertEqual(struct.unpack("<64h", audio.readframes(64)), expected_pcm)
            self.assertEqual(offset, payload_size)
            path = f"selected/{selected}.wav"
            if undone and label == "B":
                self.assertEqual(selected_paths, [])
                self.assertNotIn(path, metadata["document"])
            else:
                self.assertEqual(selected_paths, [path])
                self.assertIn(path, metadata["document"])
            states[label] = encoded[start:end]
        return states

    def test_independent_instances_restore_owned_samples_and_values(self):
        project = self.private / "qualified.rpp"
        removal = read_json(self.capture / "original-removal.json")
        self.assertEqual(removal, self.receipt["restoration"]["originalRemoval"])
        self.assertEqual(removal["firstReaperExit"], 0)
        self.assertEqual(len(removal["removedOriginals"]), 2)
        self.assertEqual(sha256(project.read_bytes()), removal["savedProjectSha256"])
        saved = self.read_project_states(project)
        restored = self.read_project_states(self.fresh / "reloaded-state.rpp")
        self.assertEqual(saved, restored, "fresh process state differs from saved state")
        self.assertEqual({label: sha256(state) for label, state in restored.items()},
                         {label: info["stateSha256"] for label, info in
                          self.receipt["restoration"]["instances"].items()})
        before = (self.first / "before-values.tsv").read_bytes()
        self.assertEqual(before, (self.fresh / "start-values.tsv").read_bytes())
        self.assertEqual(before, (PACKET / "values.tsv").read_bytes())
        self.assertEqual(len(before.splitlines()), 46)

    def test_native_gestures_and_undo_after_stopped_editor_loss(self):
        for name, expected in (("gesture-A.tsv", (0.625, 0.25, 0)),
                               ("gesture-B.tsv", (0.625, 0.375, 0)),
                               ("host-feedback.tsv", (0.4375, 0.375, 0))):
            with self.subTest(name=name):
                self.assertEqual(tuple(map(float, (self.first / name).read_text().split())), expected)
        gate = read_json(self.first / "gate-receipt.json")
        self.assertEqual(gate, self.receipt["mappingWhileStoppedAndEditorAbsent"]["gate"])
        closed = gate["closedBeforeRelease"]
        self.assertEqual(gate["hit"]["rip"], gate["executionAddress"])
        self.assertTrue(gate["hit"]["monotonicNs"] < closed["monotonicNs"]
                        < gate["releaseMonotonicNs"])
        self.assertTrue(closed["transportStopped"] and closed["workerStillTracedAndStopped"])
        for xid in (closed["nativeXid"], closed["hostFloatingXid"]):
            self.assertIn("No such window", closed["windowEvidence"][xid])
        self.assertIn("transport=0 editor=closed",
                      (self.first / "B-saved-stopped-and-closed.txt").read_text())
        completed = self.read_project_states(self.first / "B-completed-while-closed.rpp")
        undo = self.read_project_states(self.first / "B-native-undo.rpp", undone=True)
        redo = self.read_project_states(self.first / "B-native-redo.rpp")
        self.assertEqual(completed, redo)
        self.assertEqual(completed["A"], undo["A"], "B's Undo changed instance A")
        self.assertNotEqual(completed["B"], undo["B"], "Undo must remove B's mapping")

    def test_four_reaper_renders_match_independent_signed_pcm_and_automation(self):
        for stage, directory in (("before", self.first), ("after", self.fresh)):
            points = [tuple(map(float, line.split())) for line in
                      (directory / "envelope.tsv").read_text().splitlines()]
            self.assertEqual([point[1:] for point in points], [(0.5, 0), (0.25, 0), (0.25, 0)])
            self.assertEqual((points[0][0], points[2][0]), (0, 0.1))
            ramp_frames = points[1][0] * 48000
            self.assertAlmostEqual(ramp_frames, 64, delta=0 if stage == "before" else 0.001)
            for label, (_, pcm) in FIXTURES.items():
                with self.subTest(stage=stage, instance=label):
                    wave_bytes = (directory / f"{label}-{stage}.wav").read_bytes()
                    with wave.open(io.BytesIO(wave_bytes), "rb") as render:
                        self.assertEqual(render.getparams()[:4], (2, 3, 48000, FRAMES))
                        raw = render.readframes(FRAMES)
                    actual = [int.from_bytes(raw[i:i + 3], "little", signed=True)
                              for i in range(0, len(raw), 3)]
                    expected = []
                    for frame, sample in enumerate(pcm):
                        gain = 2 - min(frame / ramp_frames, 1) if label == "A" else 1.5
                        pan = (0.75, 1) if label == "A" else (1, 0.5)
                        expected.extend(sample * 256 * gain * channel_gain for channel_gain in pan)
                    expected.extend([0] * (2 * (FRAMES - len(pcm))))
                    self.assertEqual(len(actual), len(expected))
                    tolerance = 1 if (stage, label) == ("after", "A") else 0
                    for index, (observed, wanted) in enumerate(zip(actual, expected, strict=True)):
                        self.assertAlmostEqual(observed, wanted, delta=tolerance,
                                               msg=f"signed PCM at frame {index // 2}, channel {index % 2}")
                    self.assertEqual(sha256(wave_bytes),
                                     self.receipt["offlineRenders"][stage]["renders"][label]["sha256"])


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("capture", type=Path, help="directory containing the private capture and receipts")
    TriggerVst3ReaperCaptureTest.capture = parser.parse_args().capture
    unittest.main(argv=[__file__], verbosity=2)
