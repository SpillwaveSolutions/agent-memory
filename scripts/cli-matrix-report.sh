#!/usr/bin/env bash
set -euo pipefail

# Cross-CLI Matrix Report Generator
# Parses JUnit XML reports from all 5 CLIs and produces a markdown summary table.
# Usage: cli-matrix-report.sh [junit-dir]
#   Local mode:  reads $JUNIT_DIR/report-<cli>.xml
#   CI mode:     reads $JUNIT_DIR/junit-<cli>-*/report.xml

JUNIT_DIR="${1:-.}"
CLIS="claude-code gemini opencode copilot codex"

python3 - "$JUNIT_DIR" "$CLIS" <<'PYEOF'
import sys
import os
import glob
import xml.etree.ElementTree as ET
from collections import defaultdict

junit_dir = sys.argv[1]
clis = sys.argv[2].split()

# Collect per-CLI per-testcase results
# cli -> { test_name -> "PASS" | "FAIL" | "SKIP" }
cli_results = {}
all_scenarios = set()

for cli in clis:
    results = {}

    # Find XML files: CI mode (junit-<cli>-*/report.xml) or local mode (report-<cli>.xml)
    xml_files = glob.glob(os.path.join(junit_dir, f"junit-{cli}-*", "report.xml"))
    if not xml_files:
        xml_files = glob.glob(os.path.join(junit_dir, f"report-{cli}.xml"))
    if not xml_files:
        # Also try just report.xml inside a cli-named dir
        xml_files = glob.glob(os.path.join(junit_dir, cli, "report.xml"))

    for xml_file in xml_files:
        try:
            tree = ET.parse(xml_file)
            root = tree.getroot()

            # Handle both <testsuites> and <testsuite> as root
            testsuites = []
            if root.tag == "testsuites":
                testsuites = root.findall("testsuite")
            elif root.tag == "testsuite":
                testsuites = [root]

            for testsuite in testsuites:
                for testcase in testsuite.findall("testcase"):
                    name = testcase.get("name", "unknown")
                    classname = testcase.get("classname", "")
                    # Use classname: name if classname exists, else just name
                    scenario = f"{classname}: {name}" if classname else name

                    if testcase.find("skipped") is not None:
                        status = "SKIP"
                    elif testcase.find("failure") is not None or testcase.find("error") is not None:
                        status = "FAIL"
                    else:
                        status = "PASS"

                    # Worst-case merge: FAIL > SKIP > PASS
                    existing = results.get(scenario, "PASS")
                    if status == "FAIL" or existing == "FAIL":
                        results[scenario] = "FAIL"
                    elif status == "SKIP" and existing != "FAIL":
                        results[scenario] = "SKIP"
                    else:
                        results[scenario] = existing if existing != "PASS" else status

                    all_scenarios.add(scenario)
        except ET.ParseError:
            # Empty or malformed XML -- treat as 0 tests
            pass
        except Exception:
            pass

    cli_results[cli] = results

# Sort scenarios for deterministic output
sorted_scenarios = sorted(all_scenarios)

# Output markdown
print("# CLI Test Matrix Report")
print()

# Matrix table
header = "| Scenario |"
separator = "|----------|"
for cli in clis:
    header += f" {cli} |"
    separator += f" {'---':^{max(len(cli), 4)}} |"

print(header)
print(separator)

for scenario in sorted_scenarios:
    row = f"| {scenario} |"
    for cli in clis:
        results = cli_results.get(cli, {})
        status = results.get(scenario, "-")
        row += f" {status} |"
    print(row)

if not sorted_scenarios:
    print("| (no test results found) |" + " - |" * len(clis))

print()

# Summary table
print("## Summary")
print()
print("| CLI | Total | Pass | Fail | Skip |")
print("|-----|-------|------|------|------|")

total_all = 0
pass_all = 0
fail_all = 0
skip_all = 0

for cli in clis:
    results = cli_results.get(cli, {})
    total = len(results)
    passed = sum(1 for v in results.values() if v == "PASS")
    failed = sum(1 for v in results.values() if v == "FAIL")
    skipped = sum(1 for v in results.values() if v == "SKIP")
    print(f"| {cli} | {total} | {passed} | {failed} | {skipped} |")
    total_all += total
    pass_all += passed
    fail_all += failed
    skip_all += skipped

print(f"| **Total** | **{total_all}** | **{pass_all}** | **{fail_all}** | **{skip_all}** |")
print()
print(f"Overall: {pass_all} passed, {fail_all} failed, {skip_all} skipped out of {total_all} total")
PYEOF
