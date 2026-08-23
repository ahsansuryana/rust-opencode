// Generator fixture golden untuk xdg-basedir@5.1.0 (semantik || dan fallback
// homedir) yang dipakai packages/core/src/global.ts.
//
// Jalankan dari folder ini: bun run gen_xdg.ts
import { mkdirSync, writeFileSync } from "node:fs";
import { join } from "node:path";
import os from "node:os";

type Snapshot = { data?: string; config?: string; state?: string; cache?: string };

async function snapshot(env: Record<string, string | undefined>): Promise<Snapshot> {
  for (const [k, v] of Object.entries({
    XDG_DATA_HOME: undefined,
    XDG_CONFIG_HOME: undefined,
    XDG_STATE_HOME: undefined,
    XDG_CACHE_HOME: undefined,
    HOME: undefined,
    USERPROFILE: undefined,
    ...env,
  })) {
    if (v === undefined) process.env.delete?.(k) ?? delete process.env[k];
    else process.env[k] = v;
  }
  const mod = await import(`${import.meta.dir}/node_modules/xdg-basedir/index.js?bust=${Date.now()}${Math.random()}`);
  return {
    data: mod.xdgData ?? undefined,
    config: mod.xdgConfig ?? undefined,
    state: mod.xdgState ?? undefined,
    cache: mod.xdgCache ?? undefined,
  };
}

const home = os.tmpdir();
const scenarios: Record<string, Record<string, string | undefined>> = {
  defaults_from_home: { HOME: home, USERPROFILE: home },
  custom_config_home: { HOME: home, USERPROFILE: home, XDG_CONFIG_HOME: `${home}/custom-config` },
  empty_env_falls_back: { HOME: home, USERPROFILE: home, XDG_CONFIG_HOME: "" },
};

const outDir = join(import.meta.dir, "..", "..", "crates", "oc-config", "tests", "fixtures", "golden");
mkdirSync(outDir, { recursive: true });

const result: Record<string, { suffixes: Record<string, string> }> = {};
for (const [name, env] of Object.entries(scenarios)) {
  const snap = await snapshot(env);
  // simpan hanya SUFFIX relatif terhadap home supaya fixture portabel lintas mesin
  const strip = (p?: string) => (p === undefined ? "<undefined>" : p.startsWith(home) ? p.slice(home.length).replaceAll("\\", "/") : `<absolute:${p.replaceAll("\\\\", "/")}>`);
  result[name] = {
    suffixes: {
      data: strip(snap.data),
      config: strip(snap.config),
      state: strip(snap.state),
      cache: strip(snap.cache),
    },
  };
}
writeFileSync(join(outDir, "xdg.json"), JSON.stringify(result, null, 2) + "\n");
console.log(JSON.stringify(result, null, 2));
