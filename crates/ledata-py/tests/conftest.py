"""真实样例 fixture 解析：.session/fixtures 下的 .lnk 快捷方式 → PowerShell 解析真实路径。

真实数据不入仓库；路径不可用时相关用例自动跳过（skipif）。
"""
import json
import pathlib
import subprocess

import pytest

PROJECT_ROOT = pathlib.Path(__file__).resolve().parents[3]
FIXTURE_DIR = PROJECT_ROOT / ".session" / "fixtures"
RESOLVED_CACHE = FIXTURE_DIR / "resolved.json"


def _resolve_lnks() -> dict[str, str]:
    """解析 fixtures 目录下全部 .lnk，返回 {快捷方式名: 目标目录}"""
    if RESOLVED_CACHE.exists():
        return json.loads(RESOLVED_CACHE.read_text(encoding="utf-8"))
    resolved: dict[str, str] = {}
    ps = (
        "[Console]::OutputEncoding = [System.Text.Encoding]::UTF8; "
        "$sh = New-Object -ComObject WScript.Shell; "
        "Get-ChildItem -LiteralPath $env:FIXDIR -Filter *.lnk | ForEach-Object { "
        "$t = $sh.CreateShortcut($_.FullName).TargetPath; "
        "Write-Output ($_.BaseName + '|' + $t) }"
    )
    import os

    env = {**os.environ, "FIXDIR": str(FIXTURE_DIR)}
    try:
        out = subprocess.run(
            ["powershell", "-NoProfile", "-Command", ps],
            capture_output=True, text=True, timeout=30, env=env,
        ).stdout
    except Exception:
        out = ""
    for line in out.splitlines():
        if "|" in line:
            name, target = line.split("|", 1)
            resolved[name] = target
    RESOLVED_CACHE.write_text(json.dumps(resolved, ensure_ascii=False), encoding="utf-8")
    return resolved


def _fixture_dir(keyword: str) -> str | None:
    for name, target in _resolve_lnks().items():
        if keyword in name and pathlib.Path(target).is_dir():
            return target
    return None


@pytest.fixture(scope="session")
def new_format_dir() -> str:
    """新格式样例目录（...,Data,*.xlsx + ...,VASpectrum,*.CSV）"""
    d = _fixture_dir("20260703")
    if not d:
        pytest.skip("新格式真实样例不可用")
    return d


@pytest.fixture(scope="session")
def old_format_file() -> str:
    """旧格式样例文件（光谱内嵌 xlsx）"""
    d = _fixture_dir("四源&顶发射")
    if not d:
        d = _fixture_dir("0 -")
    if not d:
        pytest.skip("旧格式真实样例不可用")
    for xlsx in sorted(pathlib.Path(d).glob("*.xlsx")):
        return str(xlsx)
    pytest.skip("样例目录中没有 xlsx")


@pytest.fixture(scope="session")
def old_format_dir() -> str:
    d = _fixture_dir("四源&顶发射")
    if not d:
        pytest.skip("旧格式真实样例目录不可用")
    return d
