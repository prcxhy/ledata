"""三方对拍驱动：TS（GUI 口径基准）↔ Rust（core）↔ Python（绑定）。

前提：
- 真实样例快捷方式位于 .session/fixtures/（与 pytest conftest 同一套解析）；
- .venv 中已安装 ledata_py（maturin develop）。

用法： .venv/Scripts/python tests/golden/run_golden.py
"""
import json
import os
import pathlib
import subprocess
import sys
import tempfile

ROOT = pathlib.Path(__file__).resolve().parents[2]
sys.path.insert(0, str(ROOT / "crates" / "ledata-py" / "tests"))
from conftest import _fixture_dir  # 复用 .lnk 解析

import ledata_py as lp


def main() -> None:
    d = _fixture_dir("20260703")
    if not d:
        sys.exit("新格式真实样例不可用，无法对拍")
    xlsx = None
    chip = None
    for f in sorted(pathlib.Path(d).glob("*Data*.xlsx")):
        c, w = lp.parse_file(str(f))
        if w is None:
            xlsx, chip = f, c
            break
    if chip is None:
        sys.exit("样例中没有光谱匹配成功的文件，无法对拍")

    devices = [dev for dev in chip.devices if dev is not None]
    u_index = len(devices[0].u) // 2

    tmp = pathlib.Path(tempfile.mkdtemp(prefix="ledata_golden_"))
    payload = json.loads(chip.to_json())
    (tmp / "golden_input.json").write_text(json.dumps(payload, ensure_ascii=False), encoding="utf-8")
    (tmp / "u_index.txt").write_text(str(u_index), encoding="utf-8")
    print(f"[1/4] 输入就绪：{xlsx.name}，devices={len(devices)}，u_index={u_index}")

    subprocess.run(
        ["pnpm", "dlx", "tsx", str(ROOT / "tests" / "golden" / "golden.ts"),
         str(tmp / "golden_input.json"),
         str(tmp / "expected_performance.tsv"),
         str(tmp / "expected_spectra.tsv"),
         str(u_index)],
        check=True, cwd=ROOT,
    )
    print("[2/4] TS（GUI 口径基准）产出完成")

    env = {**os.environ, "LEDATA_GOLDEN_DIR": str(tmp)}
    subprocess.run(
        ["cargo", "test", "-p", "ledata-core", "--test", "golden", "--", "--nocapture"],
        check=True, cwd=ROOT, env=env,
    )
    print("[3/4] Rust（core）逐字符比对 PASS")

    expected_perf = (tmp / "expected_performance.tsv").read_text(encoding="utf-8")
    expected_spc = (tmp / "expected_spectra.tsv").read_text(encoding="utf-8")
    assert lp.performance_csv(devices) == expected_perf, "Python performance_csv 与 TS 不一致"
    assert lp.spectra_csv(devices, u_index) == expected_spc, "Python spectra_csv 与 TS 不一致"
    print("[4/4] Python（绑定）逐字符比对 PASS")
    print("三方对拍全部通过 ✔  产物目录：", tmp)


if __name__ == "__main__":
    main()
