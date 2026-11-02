"""Real CMake/Cargo boundary checks for managed Rust cache routing."""
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[2]
CMAKE = str(Path.home() / '.local/bin/cmake')


class CargoCacheBuildTest(unittest.TestCase):
    def test_real_cargo_reuses_dependencies_and_keeps_variants_and_coverage_correct(self):
        with tempfile.TemporaryDirectory(prefix='real-cargo-cache-') as directory:
            repo = Path(directory)
            (repo / '.ai-rules.json').write_bytes((ROOT / '.ai-rules.json').read_bytes())
            dependency = repo / 'src/dependency'
            app = repo / 'src/app'
            dependency.mkdir(parents=True)
            app.mkdir()
            (dependency / 'Cargo.toml').write_text('[package]\nname="cache_dependency"\nversion="0.1.0"\nedition="2021"\n[lib]\npath="lib.rs"\n')
            (dependency / 'lib.rs').write_text('pub fn value() -> i32 { 7 }\n')
            (app / 'Cargo.toml').write_text('[package]\nname="cache_app"\nversion="0.1.0"\nedition="2021"\n[[bin]]\nname="cache_app"\npath="main.rs"\n[features]\neditor=[]\n[dependencies]\ncache_dependency={path="../dependency"}\n')
            def source(value, name):
                (app / 'main.rs').write_text(
                    f'fn {name}() -> i32 {{ cache_dependency::value() + {value} }}\n'
                    f'fn main() {{ println!("{{}}:{{}}", {name}(), cfg!(feature="editor")); }}\n'
                    f'#[test] fn current_value() {{ assert_eq!({name}(), {7 + value}); }}\n')
            source(1, 'old_source')
            raw = str(Path.home() / '.cargo/bin/cargo')
            env = dict(os.environ, CARGO_NET_OFFLINE='true')
            subprocess.run([raw, 'generate-lockfile', '--manifest-path', str(app / 'Cargo.toml')],
                           env=env, check=True, capture_output=True)
            prefix = [sys.executable, str(ROOT / 'scripts/cargo-cache.py'), '--repo', str(repo),
                      '--cargo', raw, '--cache-root', str(repo / 'cache'), 'cargo', '--']
            def run(*args):
                result = subprocess.run([*prefix, *args, '--manifest-path', str(app / 'Cargo.toml')],
                                        env=env, capture_output=True, text=True, timeout=120)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
                return result
            def build(target, *features):
                result = run('build', '--message-format=json', '--target-dir=' + str(repo / target), *features)
                records = [json.loads(line) for line in result.stdout.splitlines()]
                artifacts = [r for r in records if r.get('reason') == 'compiler-artifact']
                executable = next(r['executable'] for r in artifacts if r.get('executable'))
                return artifacts, Path(executable)
            first, ordinary = build('ordinary')
            self.assertEqual(subprocess.check_output([ordinary], text=True).strip(), '8:false')
            repeat, _ = build('ordinary')
            self.assertTrue(all(r['fresh'] for r in repeat))
            variant, editor = build('editor', '--features', 'editor')
            self.assertEqual(subprocess.check_output([editor], text=True).strip(), '8:true')
            self.assertEqual(subprocess.check_output([ordinary], text=True).strip(), '8:false')
            report = repo / 'coverage.json'
            def ordinary_products():
                return {str(p.relative_to(repo / 'ordinary')): hashlib.sha256(p.read_bytes()).hexdigest()
                        for p in (repo / 'ordinary').rglob('*') if p.is_file()}
            published = ordinary_products()
            env['CARGO_TARGET_DIR'] = str(repo / 'ordinary')
            run('llvm-cov', '--json', '--output-path', str(report))
            self.assertIn('old_source', report.read_text())
            self.assertEqual(ordinary_products(), published,
                             'coverage must preserve the explicitly selected ordinary product tree')
            source(2, 'new_source')
            os.utime(ordinary, (2000000000, 2000000000))
            edited, ordinary = build('ordinary')
            self.assertEqual(subprocess.check_output([ordinary], text=True).strip(), '9:false')
            self.assertTrue(next(r for r in edited if r['target']['name'] == 'cache_dependency')['fresh'])
            published = ordinary_products()
            run('llvm-cov', '--json', '--output-path', str(report))
            current = report.read_text()
            self.assertIn('new_source', current)
            self.assertNotIn('old_source', current)
            self.assertEqual(ordinary_products(), published)
            warm, _ = build('ordinary')
            self.assertTrue(all(r['fresh'] for r in warm))

    def test_opt_in_mutation_registration_uses_a_separate_raw_cargo_workflow(self):
        # This executes only a logging fake, never cargo-mutants.
        with tempfile.TemporaryDirectory(prefix='cmake-mutation-registration-') as directory:
            root = Path(directory)
            calls = root / 'calls.json'
            fake = root / 'cargo'
            fake.write_text(f'#!{sys.executable}\n'
                            'import json,os,pathlib,sys\n'
                            'if sys.argv[1:]==["-vV"]:print("fixture cargo");sys.exit(0)\n'
                            f'pathlib.Path({str(calls)!r}).write_text(json.dumps({{"arguments":sys.argv[1:],'
                            '"target":os.environ.get("CARGO_TARGET_DIR"),'
                            '"build":os.environ.get("CARGO_BUILD_BUILD_DIR")}))\n')
            fake.chmod(0o755)
            build = root / 'build'
            env = dict(os.environ, DANDRUM_CARGO_CACHE_ROOT=str(root / 'cache'),
                       CARGO_BUILD_BUILD_DIR=str(root / 'ordinary-intermediates'))
            configured = subprocess.run([CMAKE, '-S', str(ROOT), '-B', str(build),
                '-DDANDRUM_ENGINE_ONLY=ON', '-DDANDRUM_ENABLE_MUTATION_CTEST=ON',
                '-DDANDRUM_RAW_CARGO=' + str(fake)], env=env, capture_output=True, text=True)
            self.assertEqual(configured.returncode, 0, configured.stdout + configured.stderr)
            result = subprocess.run([str(Path(CMAKE).with_name('ctest')), '--test-dir', str(build),
                '-R', '^rust-engine-mutation-tests$', '--output-on-failure'],
                env=env, capture_output=True, text=True)
            self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            row = json.loads(calls.read_text())
            self.assertEqual(row['arguments'][0], 'mutants')
            self.assertEqual(row['target'], str(build / 'rust-mutation-target'))
            self.assertIsNone(row['build'])

    def test_cmake_checks_cargo_freshness_even_when_existing_output_is_newer(self):
        with tempfile.TemporaryDirectory(prefix='cmake cargo cache ') as directory:
            root = Path(directory)
            home = root / 'home'
            tools = home / '.cargo/bin'
            tools.mkdir(parents=True)
            calls = root / 'calls.jsonl'
            value = root / 'value.txt'
            value.write_text('first')
            tool = tools / 'cargo'
            tool.write_text(
                f'#!{sys.executable}\n'
                'import json,os,pathlib,sys\n'
                'if sys.argv[1:]==["-vV"]:print("cargo 1.96.0");sys.exit(0)\n'
                'target=pathlib.Path(os.environ["CARGO_TARGET_DIR"])/"release"\n'
                'target.mkdir(parents=True,exist_ok=True)\n'
                f'value=pathlib.Path({str(value)!r}).read_text()\n'
                'out=target/"libdandrum_engine.a"\n'
                'if not out.exists() or out.read_text()!=value:out.write_text(value)\n'
                f'with pathlib.Path({str(calls)!r}).open("a") as f:f.write(json.dumps({{"output":str(out),"build":os.environ.get("CARGO_BUILD_BUILD_DIR")}})+"\\n")\n')
            tool.chmod(0o755)
            shutil.copy2(tool, tools / 'rustc')
            env = dict(os.environ, HOME=str(home), RUSTC=str(tools / 'rustc'),
                       DANDRUM_CARGO_CACHE_ROOT=str(root / 'cache'))
            build = root / 'build'
            def run(*args):
                result = subprocess.run([CMAKE, *args], env=env, capture_output=True, text=True, timeout=30)
                self.assertEqual(result.returncode, 0, result.stdout + result.stderr)
            run('-S', str(ROOT), '-B', str(build), '-DDANDRUM_ENGINE_ONLY=ON')
            run('--build', str(build), '--target', 'dandrum-engine-rust')
            original = json.loads(calls.read_text().splitlines()[-1])
            output = Path(original['output'])
            os.utime(output, (2000000000, 2000000000))
            value.write_text('changed')
            run('--build', str(build), '--target', 'dandrum-engine-rust')
            self.assertEqual(output.read_text(), 'changed')
            rows = [json.loads(line) for line in calls.read_text().splitlines()]
            self.assertEqual(len(rows), 2)
            self.assertEqual(rows[0]['build'], rows[1]['build'])
            self.assertIsNotNone(rows[0]['build'])


if __name__ == '__main__':
    unittest.main()
