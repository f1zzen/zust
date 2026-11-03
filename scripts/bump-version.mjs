#!/usr/bin/env node
// Версия Zust хранится в четырёх местах — этот скрипт держит их одинаковыми.
//
//   npm run bump 2.2.0     — выставить версию везде
//   npm run check:version  — проверить, что версии совпадают (используется в CI)
//
// Файл `version` в корне читают уже установленные Zust для проверки обновлений,
// поэтому его нужно менять (и пушить в main) только вместе с выпуском релиза.

import { readFileSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const file = (p) => join(root, p);

const read = {
    "package.json": () => JSON.parse(readFileSync(file("package.json"), "utf8")).version,
    "src-tauri/Cargo.toml": () => readFileSync(file("src-tauri/Cargo.toml"), "utf8").match(/^version = "([^"]+)"/m)?.[1],
    "src-tauri/Cargo.lock": () =>
        readFileSync(file("src-tauri/Cargo.lock"), "utf8").match(/\[\[package\]\]\nname = "zust"\nversion = "([^"]+)"/)?.[1],
    version: () => readFileSync(file("version"), "utf8").trim(),
};

const write = {
    "package.json": (v) => {
        const pkg = JSON.parse(readFileSync(file("package.json"), "utf8"));
        pkg.version = v;
        writeFileSync(file("package.json"), JSON.stringify(pkg, null, 2) + "\n");
    },
    "src-tauri/Cargo.toml": (v) => {
        const s = readFileSync(file("src-tauri/Cargo.toml"), "utf8");
        writeFileSync(file("src-tauri/Cargo.toml"), s.replace(/^version = "[^"]+"/m, `version = "${v}"`));
    },
    "src-tauri/Cargo.lock": (v) => {
        const s = readFileSync(file("src-tauri/Cargo.lock"), "utf8");
        writeFileSync(
            file("src-tauri/Cargo.lock"),
            s.replace(/(\[\[package\]\]\nname = "zust"\nversion = )"[^"]+"/, `$1"${v}"`)
        );
    },
    version: (v) => writeFileSync(file("version"), v + "\n"),
};

const arg = process.argv[2];

if (!arg || arg === "--check") {
    const versions = Object.fromEntries(Object.entries(read).map(([k, f]) => [k, f()]));
    const unique = new Set(Object.values(versions));
    for (const [k, v] of Object.entries(versions)) console.log(`${v ?? "?"}\t${k}`);
    if (unique.size !== 1) {
        console.error("\nВерсии не совпадают. Выполните: npm run bump <версия>");
        process.exit(1);
    }
    console.log(`\nOK: все версии ${[...unique][0]}`);
    process.exit(0);
}

if (!/^\d+\.\d+\.\d+(-[0-9A-Za-z.]+)?$/.test(arg)) {
    console.error(`Некорректная версия: ${arg} (ожидается, например, 2.2.0)`);
    process.exit(1);
}
for (const [k, f] of Object.entries(write)) {
    f(arg);
    console.log(`${arg}\t${k}`);
}
