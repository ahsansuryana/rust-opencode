// Generator fixture golden untuk jsonc-parser@3.3.1 (parse + errors),
// algoritma yang diport ke crates/oc-config/src/parse.rs.
//
// Jalankan dari folder ini: bun run gen_jsonc.ts
import { parse, printParseErrorCode } from "jsonc-parser";
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";

const cases: Record<string, string> = {
  plain_json: `{"a":1,"b":[true,null,"x"],"n":-5,"f":2.5}`,
  comments_and_trailing_comma: `{\n  // line comment\n  /* block */\n  "a": 1,\n  "b": [1, 2,],\n}`,
  nested_objects: `{"mode":{"plan":{"prompt":"p"}},"tools":{"write":false},"autoshare":true}`,
  trailing_garbage: `{} x`,
  bad_symbol: `{\n  "a": ?,\n}`,
  unterminated_string: `{"a": "unterminated}`,
  empty_input: ``,
  missing_colon: `{"a" 1}`,
  missing_comma: `{"a": 1 "b": 2}`,
  unclosed_brace: `{"a": {"b": 1}`,
  unclosed_bracket: `{"a": [1, 2`,
  leading_zero: `{"a": 01}`,
  bad_number_dot: `{"b": 1.}`,
  bad_escape: `{"s": "\\q"}`,
  keyword_true_false_null: `[true,false,null]`,
  duplicate_keys: `{"a":1,"a":2}`,
  config_like: `{\n  "$schema": "https://opencode.ai/config.json",\n  // preferensi\n  "model": "anthropic/claude-3",\n  "permission": { "bash": { "rm*": "deny" } },\n}`,
};

const outDir = join(import.meta.dir, "..", "..", "crates", "oc-config", "tests", "fixtures", "golden", "jsonc");
mkdirSync(outDir, { recursive: true });

for (const [name, text] of Object.entries(cases)) {
  const errors: { error: number; offset: number; length: number }[] = [];
  const value = parse(text, errors, { allowTrailingComma: true });
  writeFileSync(
    join(outDir, `${name}.json`),
    JSON.stringify(
      {
        input: text,
        value: value === undefined ? null : value,
        value_undefined: value === undefined,
        errors: errors.map((e) => ({ code: printParseErrorCode(e.error), offset: e.offset })),
      },
      null,
      2,
    ) + "\n",
  );
  console.log(`ok ${name} (${errors.length} errors)`);
}
