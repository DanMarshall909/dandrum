import importlib.util
from pathlib import Path
import struct
import tempfile
import unittest
import zlib

ROOT = Path(__file__).resolve().parents[2]


class NativeCaptureTest(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        spec = importlib.util.spec_from_file_location('native_sampler_capture', ROOT / 'scripts/capture-advanced-sampler.py')
        cls.capture = importlib.util.module_from_spec(spec)
        spec.loader.exec_module(cls.capture)

    def test_all_required_states_are_captured_at_their_actual_design_sizes(self):
        rows = self.capture.capture_cases()
        self.assertEqual(len({r['state'] for r in rows if r['kind'] == 'state'}), 30)
        by_state = {r['state']: r for r in rows if r['kind'] == 'state'}
        self.assertEqual(by_state['min']['size'], 'min')
        self.assertEqual(by_state['exp']['size'], 'expanded')
        self.assertEqual(by_state['sample']['size'], 'default')
        themes = [r for r in rows if r['kind'] == 'theme']
        self.assertEqual(len(themes), 16)
        self.assertEqual({r['finish'] for r in themes}, {'soft', 'flat'})

    def test_native_snapshot_conversion_preserves_signed_byte_color_and_alpha(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'source.pam'; target = Path(directory) / 'target.png'
            pixels = bytes([2, 128, 255, 0, 250, 0, 10, 255])
            source.write_bytes(b'P7\nWIDTH 2\nHEIGHT 1\nDEPTH 4\nMAXVAL 255\nTUPLTYPE RGB_ALPHA\nENDHDR\n' + pixels)
            self.assertEqual(self.capture.snapshot_to_png(source, target), (2, 1))
            data = target.read_bytes(); self.assertEqual(data[:8], b'\x89PNG\r\n\x1a\n')
            chunks = {}; offset = 8
            while offset < len(data):
                length = struct.unpack('>I', data[offset:offset+4])[0]
                kind = data[offset+4:offset+8]; payload = data[offset+8:offset+8+length]
                self.assertEqual(struct.unpack('>I', data[offset+8+length:offset+12+length])[0], zlib.crc32(kind + payload))
                chunks[kind] = payload; offset += 12 + length
            self.assertEqual(struct.unpack('>II', chunks[b'IHDR'][:8]), (2, 1))
            self.assertEqual(zlib.decompress(chunks[b'IDAT']), b'\0' + pixels)

    def test_complete_size_matrix_captures_every_workspace_at_all_three_sizes(self):
        rows = self.capture.capture_cases(all_sizes=True)
        pages = [r for r in rows if r['kind'] == 'page-size']
        required = {'empty', 'pads', 'macros', 'sample', 'slices', 'mapping', 'vel',
                    'voice', 'mod', 'routing', 'fx', 'overview', 'diag'}
        self.assertEqual({(r['state'], r['size']) for r in pages},
                         {(page, size) for page in required for size in ['min', 'default', 'expanded']})
        self.assertEqual(len(pages), 39)
        self.assertEqual(len(rows), 85)
        self.assertEqual(len({r['name'] for r in rows}), len(rows))
        self.assertTrue(all(r['theme'] == 'aluminium' and r['finish'] == 'soft' for r in pages))
        self.assertEqual([r for r in rows if r['kind'] != 'page-size'], self.capture.capture_cases())

    def test_fractional_scale_changes_render_pixels_without_resizing_evidence(self):
        environment = {'SLINT_SCALE_FACTOR': '2', 'DISPLAY': ':99'}
        scaled = self.capture.capture_environment(environment, 1.25)
        self.assertEqual(scaled['SLINT_SCALE_FACTOR'], '1.25')
        self.assertEqual(scaled['DISPLAY'], ':99')
        self.assertEqual(environment['SLINT_SCALE_FACTOR'], '2')
        self.assertEqual(self.capture.capture_environment(environment, None), environment)
        for invalid in [0, -1, float('nan'), float('inf')]:
            with self.assertRaises(ValueError): self.capture.capture_environment(environment, invalid)

    def test_incomplete_native_snapshots_fail_instead_of_producing_evidence(self):
        with tempfile.TemporaryDirectory() as directory:
            source = Path(directory) / 'bad.pam'; target = Path(directory) / 'bad.png'
            source.write_bytes(b'P7\nWIDTH 2\nHEIGHT 1\nDEPTH 4\nMAXVAL 255\nENDHDR\n\0')
            with self.assertRaises(ValueError): self.capture.snapshot_to_png(source, target)
            self.assertFalse(target.exists())


if __name__ == '__main__': unittest.main()
