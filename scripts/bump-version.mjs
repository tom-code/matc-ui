import { readFileSync, writeFileSync } from 'fs';
import { execSync } from 'child_process';
import { resolve, dirname } from 'path';
import { fileURLToPath } from 'url';

const root = resolve(dirname(fileURLToPath(import.meta.url)), '..');

const version = process.argv[2];
if (!version || !/^\d+\.\d+\.\d+(-[\w.]+)?$/.test(version)) {
  console.error('Usage: npm run version:set -- <x.y.z>');
  process.exit(1);
}

const pkgPath = resolve(root, 'package.json');
const pkg = JSON.parse(readFileSync(pkgPath, 'utf8'));
pkg.version = version;
writeFileSync(pkgPath, JSON.stringify(pkg, null, 2) + '\n');
console.log(`  package.json              -> ${version}`);

const tauriConfPath = resolve(root, 'src-tauri/tauri.conf.json');
const tauriConf = JSON.parse(readFileSync(tauriConfPath, 'utf8'));
tauriConf.version = version;
writeFileSync(tauriConfPath, JSON.stringify(tauriConf, null, 2) + '\n');
console.log(`  src-tauri/tauri.conf.json -> ${version}`);

const cargoPath = resolve(root, 'src-tauri/Cargo.toml');
let cargo = readFileSync(cargoPath, 'utf8');
let matched = false;
cargo = cargo.replace(/^version = ".*?"$/m, () => {
  matched = true;
  return `version = "${version}"`;
});
if (!matched) {
  console.error('ERROR: could not find version line in src-tauri/Cargo.toml');
  process.exit(1);
}
writeFileSync(cargoPath, cargo);
console.log(`  src-tauri/Cargo.toml      -> ${version}`);

try {
  execSync('cargo update -p matc-ui --manifest-path src-tauri/Cargo.toml', {
    cwd: root,
    stdio: 'pipe',
  });
  console.log('  Cargo.lock updated');
} catch {
  console.log('  Note: run "cargo update -p matc-ui --manifest-path src-tauri/Cargo.toml" to sync Cargo.lock');
}
