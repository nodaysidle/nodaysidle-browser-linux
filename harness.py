#!/usr/bin/env python3
"""
Product Harness Orchestrator for Claude Code
Works in ANY repository.
Manages the 3-role lifecycle: PLANNER -> BUILDER -> EVALUATOR
Enforces independent evaluation contexts, git-anchored evidence, and repair caps.
"""

import argparse
import datetime
import json
import os
import re
import shutil
import subprocess
import sys
from pathlib import Path

# The repository where harness is executed
CWD = Path.cwd()
# The global template source directory
TEMPLATE_DIR = Path("/home/arch/dev/nodaysidle/product-harness")

def get_git_revision() -> str:
    try:
        res = subprocess.run(
            ["git", "rev-parse", "--short", "HEAD"],
            cwd=CWD,
            capture_output=True,
            text=True,
            check=True
        )
        return res.stdout.strip()
    except Exception:
        return "UNTRACKED_NO_GIT"

def get_run_id() -> str:
    timestamp = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    git_rev = get_git_revision()
    return f"{timestamp}_{git_rev}"

def ensure_artifacts_dir(run_id: str) -> Path:
    art_dir = CWD / "artifacts" / run_id
    art_dir.mkdir(parents=True, exist_ok=True)
    return art_dir

def read_text(path: Path) -> str:
    if path.is_file():
        return path.read_text(encoding="utf-8")
    return ""

def write_text(path: Path, content: str) -> None:
    path.write_text(content, encoding="utf-8")

def load_features() -> dict:
    feat_path = CWD / "FEATURES.json"
    if feat_path.is_file():
        try:
            return json.loads(feat_path.read_text(encoding="utf-8"))
        except Exception:
            pass
    return {"features": []}

def get_active_feature(features_data: dict) -> dict:
    for f in features_data.get("features", []):
        if f.get("status") in ["todo", "in_progress", "repairing"]:
            return f
    return {}

def show_status():
    print("=" * 60)
    print(" PRODUCT HARNESS STATUS")
    print("=" * 60)
    print(f"Directory:    {CWD}")
    print(f"Git Revision: {get_git_revision()}")
    
    feats = load_features()
    active = get_active_feature(feats)
    if active:
        print(f"Active Feature: [{active.get('id')}] {active.get('title')} (Status: {active.get('status')})")
    else:
        print("Active Feature: None pending (All features completed or backlog empty)")
    
    handoff_text = read_text(CWD / "HANDOFF.md")
    cap_match = re.search(r"Attempts used / cap:\s*(\d+)\s*/\s*(\d+)", handoff_text)
    if cap_match:
        print(f"Repair Attempts: {cap_match.group(1)} / {cap_match.group(2)}")
    
    print("\nFeature Backlog:")
    features_list = feats.get("features", [])
    if not features_list:
        print(" (No features found. Run `harness plan` to initialize backlog)")
    for f in features_list:
        status = f.get("status", "unknown").upper()
        rev = f.get("code_revision") or "none"
        print(f" - [{status:<10}] {f.get('id')}: {f.get('title')} (rev: {rev})")
    
    print("=" * 60)

def run_claude(prompt: str, interactive: bool = True, append_system_prompt: str = "") -> subprocess.CompletedProcess:
    cmd = ["claude"]
    if append_system_prompt:
        cmd.extend(["--append-system-prompt", append_system_prompt])
    
    if interactive:
        cmd.append(prompt)
        print(f"\n[Harness] Launching interactive Claude Code session in {CWD.name}...")
        return subprocess.run(cmd, cwd=CWD)
    else:
        cmd.extend(["-p", prompt])
        print(f"\n[Harness] Running Claude in print mode...")
        res = subprocess.run(cmd, cwd=CWD, capture_output=True, text=True)
        return res

def cmd_init():
    print(f"\n>>> Initializing Product Harness in: {CWD} <<<")
    # 1. Copy roles
    roles_dest = CWD / "roles"
    roles_src = TEMPLATE_DIR / "roles"
    if not roles_dest.exists() and roles_src.exists():
        shutil.copytree(roles_src, roles_dest)
        print(" [✓] Copied roles/ (PLANNER.md, BUILDER.md, EVALUATOR.md)")
    
    # 2. Template files
    for fname in ["PRODUCT.md", "CONTRACT.md", "FEATURES.json", "HANDOFF.md"]:
        dest = CWD / fname
        src = TEMPLATE_DIR / fname
        if not dest.exists() and src.exists():
            shutil.copyfile(src, dest)
            print(f" [✓] Created {fname}")
    
    # 3. Update .gitignore
    gitignore = CWD / ".gitignore"
    if gitignore.exists():
        gi_content = read_text(gitignore)
        if "artifacts/" not in gi_content:
            write_text(gitignore, gi_content.rstrip() + "\n/artifacts/\n")
            print(" [✓] Added /artifacts/ to .gitignore")
    else:
        write_text(gitignore, "/artifacts/\n")
        print(" [✓] Created .gitignore with /artifacts/")
    
    # 4. Check CLAUDE.md
    claude_md = CWD / "CLAUDE.md"
    harness_rules = (
        "\n## Product Harness & Verification Discipline\n"
        "- Read PRODUCT.md for high-level goals, scope boundaries, and verified commands.\n"
        "- Read CONTRACT.md for the active feature's strict checks and evidence criteria.\n"
        "- Read FEATURES.json and HANDOFF.md when starting or resuming work.\n"
        "- Build one agreed feature at a time; preserve unrelated code.\n"
        "- Save full verification logs under artifacts/<run-id>/.\n"
        "- Mark a feature PASS in FEATURES.json ONLY after the independent EVALUATOR verifies evidence.\n"
        "- Repair rounds are capped at 2 before stopping for human review.\n"
    )
    if claude_md.exists():
        content = read_text(claude_md)
        if "Product Harness" not in content:
            write_text(claude_md, content.rstrip() + "\n" + harness_rules)
            print(" [✓] Appended harness discipline to existing CLAUDE.md")
    else:
        write_text(claude_md, "# CLAUDE.md\n" + harness_rules)
        print(" [✓] Created CLAUDE.md with harness discipline")
    
    print("\nInitialization complete! Run `harness status` or `harness plan` to begin.")

def cmd_plan(interactive: bool = True):
    print("\n>>> Launching PLANNER Role <<<")
    planner_prompt = (
        "You are operating in the PLANNER role defined in roles/PLANNER.md.\n"
        "Read PRODUCT.md and CLAUDE.md.\n"
        "Review the target scope, settle architecture decisions, update FEATURES.json, "
        "and produce or update CONTRACT.md for the active feature.\n"
        "Ensure checks in CONTRACT.md have explicit Action, Expected observation, and required Evidence."
    )
    role_content = read_text(CWD / "roles" / "PLANNER.md")
    run_claude(planner_prompt, interactive=interactive, append_system_prompt=role_content)

def cmd_build(interactive: bool = True):
    run_id = get_run_id()
    art_dir = ensure_artifacts_dir(run_id)
    print(f"\n>>> Launching BUILDER Role (Run ID: {run_id}) <<<")
    
    builder_prompt = (
        f"You are operating in the BUILDER role defined in roles/BUILDER.md.\n"
        f"Current Git Revision: {get_git_revision()}\n"
        f"Artifacts Directory: {art_dir.relative_to(CWD)}\n"
        "1. Read CONTRACT.md and HANDOFF.md.\n"
        "2. Implement the active feature.\n"
        "3. Run the project verification commands (syntax, build, unit tests).\n"
        f"4. Save full command outputs and logs under artifacts/{run_id}/.\n"
        "5. Update HANDOFF.md with code changes, actual test results, and current git hash.\n"
        "Do NOT mark the feature pass in FEATURES.json; that is reserved exclusively for the EVALUATOR."
    )
    role_content = read_text(CWD / "roles" / "BUILDER.md")
    res = run_claude(builder_prompt, interactive=interactive, append_system_prompt=role_content)
    if not interactive and res.stdout:
        print(res.stdout)
        write_text(art_dir / "builder_log.txt", res.stdout)

def cmd_eval(interactive: bool = True):
    run_id = get_run_id()
    git_rev = get_git_revision()
    art_dir = ensure_artifacts_dir(run_id)
    print(f"\n>>> Launching EVALUATOR Role in ISOLATED CONTEXT (Run ID: {run_id}) <<<")
    
    eval_prompt = (
        f"You are operating in the EVALUATOR role defined in roles/EVALUATOR.md.\n"
        f"Current Git Revision: {git_rev}\n"
        f"Artifacts Directory: {art_dir.relative_to(CWD)}\n"
        "CRITICAL: Do NOT rely on builder promises or self-ratings. You must independently verify.\n"
        "1. Read CONTRACT.md to inspect the required checks.\n"
        "2. Run the actual project test suite or inspection commands directly (e.g. `cargo test -- --test-threads=1`, `cargo clippy`).\n"
        "3. Save test evidence into artifacts/{run_id}/eval_evidence.txt.\n"
        "4. For each check in CONTRACT.md, determine: PASS, FAIL, or UNVERIFIED.\n"
        "5. If all checks pass with concrete evidence, update CONTRACT.md (Evidence column and completion) and FEATURES.json (status: 'done', code_revision: current hash).\n"
        "6. If any check fails, record the concrete reproduction steps and failures in HANDOFF.md and CONTRACT.md."
    )
    role_content = read_text(CWD / "roles" / "EVALUATOR.md")
    res = run_claude(eval_prompt, interactive=interactive, append_system_prompt=role_content)
    if not interactive and res.stdout:
        print(res.stdout)
        write_text(art_dir / "eval_log.txt", res.stdout)

def cmd_repair(interactive: bool = True):
    handoff_path = CWD / "HANDOFF.md"
    content = read_text(handoff_path)
    cap_match = re.search(r"Attempts used / cap:\s*(\d+)\s*/\s*(\d+)", content)
    attempts = 0
    cap = 2
    if cap_match:
        attempts = int(cap_match.group(1))
        cap = int(cap_match.group(2))
    
    if attempts >= cap:
        print(f"\n[ALERT] Repair cap reached ({attempts}/{cap})! Stopping for human review.")
        print("Review the failure logs in artifacts/ and HANDOFF.md before manually resetting the attempt counter.")
        return
    
    new_attempts = attempts + 1
    new_content = re.sub(
        r"Attempts used / cap:\s*\d+\s*/\s*\d+",
        f"Attempts used / cap: {new_attempts} / {cap}",
        content
    )
    write_text(handoff_path, new_content)
    print(f"\n>>> Starting Repair Round {new_attempts}/{cap} <<<")
    
    repair_prompt = (
        f"You are in the BUILDER role fixing failures reported by the EVALUATOR.\n"
        f"Repair Attempt: {new_attempts} / {cap}\n"
        "Read the failed checks and reproduction steps in HANDOFF.md and CONTRACT.md.\n"
        "Fix the underlying bugs. Re-run verification checks. Update HANDOFF.md when ready for re-evaluation."
    )
    role_content = read_text(CWD / "roles" / "BUILDER.md")
    run_claude(repair_prompt, interactive=interactive, append_system_prompt=role_content)

def main():
    parser = argparse.ArgumentParser(description="Product Harness CLI Orchestrator")
    subparsers = parser.add_subparsers(dest="command")
    
    subparsers.add_parser("init", help="Initialize product harness templates in the current repo")
    subparsers.add_parser("status", help="Show current product harness status and backlog")
    
    p_plan = subparsers.add_parser("plan", help="Launch Planner role to define/update contracts")
    p_plan.add_argument("--print", dest="interactive", action="store_false", help="Run non-interactively")
    
    p_build = subparsers.add_parser("build", help="Launch Builder role to implement feature")
    p_build.add_argument("--print", dest="interactive", action="store_false", help="Run non-interactively")
    
    p_eval = subparsers.add_parser("eval", help="Launch Evaluator role in fresh context to verify checks")
    p_eval.add_argument("--print", dest="interactive", action="store_false", help="Run non-interactively")
    
    p_repair = subparsers.add_parser("repair", help="Run repair cycle with failure feedback capped at 2 rounds")
    p_repair.add_argument("--print", dest="interactive", action="store_false", help="Run non-interactively")

    args = parser.parse_args()
    if not args.command or args.command == "status":
        show_status()
    elif args.command == "init":
        cmd_init()
    elif args.command == "plan":
        cmd_plan(interactive=args.interactive)
    elif args.command == "build":
        cmd_build(interactive=args.interactive)
    elif args.command == "eval":
        cmd_eval(interactive=args.interactive)
    elif args.command == "repair":
        cmd_repair(interactive=args.interactive)

if __name__ == "__main__":
    main()
