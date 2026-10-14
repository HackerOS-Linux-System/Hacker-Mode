import { readFileSync, readdirSync } from "node:fs";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const root = join(dirname(fileURLToPath(import.meta.url)), "..");
const dir = join(root, "src/i18n/locales");
const load = (f) => JSON.parse(readFileSync(join(dir, f), "utf8"));
const placeholders = (s) => [...s.matchAll(/\{(\w+)\}/g)].map((m) => m[1]).sort().join(",");

const files = readdirSync(dir).filter((f) => f.endsWith(".json")).sort();
const en = load("en.json");
const indexSrc = readFileSync(join(root, "src/i18n/index.ts"), "utf8");
const problems = [];

for (const file of files) {
  const lang = file.replace(".json", "");
  if (!indexSrc.includes(`./locales/${file}`)) problems.push(`${lang}: plik nie jest zaimportowany w i18n/index.ts`);
  const data = load(file);
  for (const key of Object.keys(en)) {
    if (!(key in data)) problems.push(`${lang}: brakuje klucza "${key}"`);
    else {
      if (typeof data[key] !== "string" || data[key].trim() === "") problems.push(`${lang}: pusta wartość "${key}"`);
      else if (placeholders(data[key]) !== placeholders(en[key]))
        problems.push(`${lang}: "${key}" ma inne placeholdery (${placeholders(data[key]) || "brak"}) niż en (${placeholders(en[key]) || "brak"})`);
    }
  }
  for (const key of Object.keys(data)) if (!(key in en)) problems.push(`${lang}: nadmiarowy klucz "${key}"`);
}

if (problems.length) {
  console.error(problems.join("\n"));
  console.error(`\ni18n: ${problems.length} problem(ów)`);
  process.exit(1);
}
console.log(`i18n OK: ${files.length} języków × ${Object.keys(en).length} kluczy`);
