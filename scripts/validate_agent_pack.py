#!/usr/bin/env python3
"""Validate agent-pack manifests, discoverability, and MCP tool contracts."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path
from typing import Any

ROOT = Path(__file__).resolve().parents[1]
AGENT_PACK = ROOT / "agent-pack"


def fail(msg: str) -> None:
    print(f"[agent-pack:fail] {msg}", file=sys.stderr)
    raise SystemExit(1)


def info(msg: str) -> None:
    print(f"[agent-pack] {msg}")


def load_json(path: Path) -> Any:
    try:
        return json.loads(path.read_text(encoding="utf-8"))
    except Exception as exc:  # noqa: BLE001
        fail(f"unable to parse JSON at {path}: {exc}")


def assert_file(rel: str) -> Path:
    p = AGENT_PACK / rel
    if not p.exists() or not p.is_file():
        fail(f"missing file: agent-pack/{rel}")
    return p


def assert_dir(rel: str) -> Path:
    p = AGENT_PACK / rel
    if not p.exists() or not p.is_dir():
        fail(f"missing directory: agent-pack/{rel}")
    return p


def run_mcp(request: dict[str, Any]) -> dict[str, Any]:
    proc = subprocess.run(
        [str(AGENT_PACK / "bin" / "puml-mcp")],
        input=json.dumps(request) + "\n",
        text=True,
        capture_output=True,
        cwd=ROOT,
        check=False,
    )
    if proc.returncode != 0:
        fail(f"mcp runner exit={proc.returncode}: {proc.stderr.strip()}")
    lines = [ln for ln in proc.stdout.splitlines() if ln.strip()]
    if not lines:
        fail("mcp runner returned empty response")
    try:
        return json.loads(lines[-1])
    except json.JSONDecodeError as exc:
        fail(f"mcp response is not valid JSON: {exc}: {lines[-1]}")


def validate_plugin_manifest(path: Path, mcp_path: str, require_agents: bool) -> None:
    doc = load_json(path)
    for key in ["name", "version", "description", "skills", "mcp", "marketplace"]:
        if key not in doc:
            fail(f"{path.name} missing key '{key}'")
    if doc["mcp"] != mcp_path:
        fail(f"{path.name} mcp points to {doc['mcp']} expected {mcp_path}")
    if not isinstance(doc["skills"], list) or not doc["skills"]:
        fail(f"{path.name} skills must be a non-empty list")
    for skill_rel in doc["skills"]:
        skill_dir = AGENT_PACK / skill_rel
        if not skill_dir.is_dir():
            fail(f"{path.name} references missing skill dir: {skill_rel}")
        if not (skill_dir / "SKILL.md").is_file():
            fail(f"{path.name} skill missing SKILL.md: {skill_rel}")
    if require_agents:
        agents = doc.get("agents")
        if not isinstance(agents, list) or not agents:
            fail(f"{path.name} must include a non-empty agents list")
        for agent_rel in agents:
            if not (AGENT_PACK / agent_rel).is_file():
                fail(f"{path.name} references missing agent file: {agent_rel}")

    marketplace_rel = doc["marketplace"]
    marketplace_path = AGENT_PACK / marketplace_rel
    if not marketplace_path.is_file():
        fail(f"{path.name} marketplace not found: {marketplace_rel}")


def validate_mcp_contract() -> None:
    mcp_doc = load_json(assert_file(".mcp.json"))
    for key in ["name", "version", "transport", "tools"]:
        if key not in mcp_doc:
            fail(f".mcp.json missing key '{key}'")
    if mcp_doc["transport"] != "stdio":
        fail(".mcp.json transport must be 'stdio'")
    if not isinstance(mcp_doc["tools"], list) or not mcp_doc["tools"]:
        fail(".mcp.json tools must be non-empty list")

    tools_by_name: dict[str, dict[str, Any]] = {}
    for tool in mcp_doc["tools"]:
        name = tool.get("name")
        if not isinstance(name, str) or not name:
            fail(".mcp.json tool has missing/invalid name")
        if name in tools_by_name:
            fail(f"duplicate tool in .mcp.json: {name}")
        tools_by_name[name] = tool

    init_resp = run_mcp({"jsonrpc": "2.0", "id": 1, "method": "initialize", "params": {}})
    if init_resp.get("result", {}).get("protocolVersion") != "2025-06-18":
        fail("initialize protocolVersion mismatch")

    tools_resp = run_mcp({"jsonrpc": "2.0", "id": 2, "method": "tools/list", "params": {}})
    runtime_tools = tools_resp.get("result", {}).get("tools")
    if not isinstance(runtime_tools, list):
        fail("tools/list did not return result.tools list")

    runtime_names = {tool.get("name") for tool in runtime_tools if isinstance(tool, dict)}
    spec_names = set(tools_by_name.keys())
    if runtime_names != spec_names:
        fail(f"tool name mismatch .mcp.json={sorted(spec_names)} runtime={sorted(runtime_names)}")

    for rt in runtime_tools:
        name = rt["name"]
        spec = tools_by_name[name]
        spec_input = spec.get("input", {})
        runtime_input = rt.get("inputSchema", {})
        if spec_input.get("additionalProperties", True) != runtime_input.get("additionalProperties", True):
            fail(f"tool '{name}' additionalProperties mismatch between spec and runtime")
        spec_props = set((spec_input.get("properties") or {}).keys())
        runtime_props = set((runtime_input.get("properties") or {}).keys())
        if spec_props != runtime_props:
            fail(f"tool '{name}' properties mismatch spec={sorted(spec_props)} runtime={sorted(runtime_props)}")


def main() -> int:
    info("validating filesystem layout")
    assert_dir("skills")
    assert_dir("agents")
    assert_file("README.md")
    assert_file("bin/puml-mcp")

    info("validating plugin manifests")
    validate_plugin_manifest(assert_file(".codex-plugin/plugin.json"), ".mcp.json", require_agents=False)
    validate_plugin_manifest(assert_file(".claude-plugin/plugin.json"), ".mcp.json", require_agents=True)

    info("validating marketplace metadata")
    for rel in [".codex-plugin/marketplace.json", ".claude-plugin/marketplace.json"]:
        doc = load_json(assert_file(rel))
        for key in ["slug", "display_name", "short_description", "categories", "tags", "publisher", "visibility"]:
            if key not in doc:
                fail(f"{rel} missing key '{key}'")

    info("validating MCP contract and runtime tool surface")
    validate_mcp_contract()

    info("validation complete")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
