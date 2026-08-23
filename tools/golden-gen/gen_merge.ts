// Generator fixture golden untuk semantik remeda.mergeDeep@2.26.0
// (yang dipakai config.ts lewat mergeConfig).
//
// Jalankan dari folder ini: bun run gen_merge.ts
import { mergeDeep } from "remeda";
import { mkdirSync, writeFileSync } from "node:fs";
import { dirname, join } from "node:path";

const cases = [
  ["scalar_override", [{ a: 1, keep: true }, { a: 2 }]],
  ["nested_recurse", [{ a: { x: 1, y: 2 }, b: 1 }, { a: { y: 9, z: 3 } }]],
  ["array_replaced_not_merged", [{ list: [1, 2, 3] }, { list: [9] }]],
  ["object_over_array", [{ v: [1] }, { v: { k: 1 } }]],
  ["array_over_object", [{ v: { k: 1 } }, { v: [7] }]],
  ["null_wins", [{ v: { deep: 1 } }, { v: null }]],
  ["new_keys_appended_in_order", [{ first: 1, mid: 2 }, { mid: 3, last: 4 }]],
  ["deep_three_levels", [{ a: { b: { c: 1, d: 2 } } }, { a: { b: { d: 20, e: 30 } } }]],
  [
    "config_like",
    [
      { model: "g/model", instructions: ["a.md"], permission: { edit: "ask" } },
      { model: "p/model", instructions: ["b.md"], permission: { bash: { rm: "deny" } } },
    ],
  ],
];

const outDir = join(import.meta.dir, "..", "..", "crates", "oc-config", "tests", "fixtures", "golden", "merge");
mkdirSync(outDir, { recursive: true });

for (const [name, [destination, source]] of cases) {
  const merged = mergeDeep(destination, source);
  writeFileSync(
    join(outDir, `${name}.json`),
    JSON.stringify({ target: destination, source, merged }, null, 2) + "\n",
  );
  console.log(`ok ${name}`);
}
