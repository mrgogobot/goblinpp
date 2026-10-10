#!/usr/bin/env python3
"""Opt-in real-file/RSS batch stress test (several GiB of free disk required).

Runs the installed or built engine, never changes scientific user data. Generated
inputs/evidence live in a dedicated temporary directory and are removed afterward.
Linux/macOS peak RSS is measured for each child separately, not Python's process.
This is a measured regression gate, not enforcement of a general process quota.
"""
import argparse
import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile


def digest(path):
    h = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(1024 * 1024), b""):
            h.update(block)
    return h.hexdigest()


def measured(command, directory, label):
    with (directory / f"{label}.out").open("wb") as out, (directory / f"{label}.err").open("wb") as err:
        child = subprocess.Popen(command, cwd=directory, stdout=out, stderr=err)
        _, status, usage = os.wait4(child.pid, 0)
        child.returncode = os.waitstatus_to_exitcode(status)
    if child.returncode:
        raise RuntimeError((directory / f"{label}.err").read_text())
    return int(usage.ru_maxrss * (1 if sys.platform == "darwin" else 1024))


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("engine", type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--large", action="store_true", help="Include a real input larger than 1 GiB")
    args = parser.parse_args()
    if args.report.exists():
        raise SystemExit("Refusing to overwrite a measured report")
    engine = args.engine.resolve()
    version = subprocess.check_output([str(engine), "--version"], text=True).strip()
    targets = [2 * 1024**2, 64 * 1024**2] + ([1024**3 + 2 * 1024**2] if args.large else [])
    results = []
    with tempfile.TemporaryDirectory(prefix="goblin-batch-stress-") as temporary:
        root = Path(temporary)
        for size in targets:
            directory = root / str(size)
            directory.mkdir()
            header = b"id,value\n"
            row = b"18446744073709551615," + b"x" * 1024 + b"\n"
            rows = (size - len(header)) // len(row) + 1
            block = row * 1024
            with (directory / "data.csv").open("xb") as stream:
                stream.write(header)
                for _ in range(rows // 1024):
                    stream.write(block)
                stream.write(row * (rows % 1024))
            source = directory / "stress.gbl"
            source.write_text('''GO_PARANOID
reader = csv_batch_open("data.csv", ["id", "value"], 4096, 8388608, 2147483648)
writer = csv_stream_open("copy.csv", ["id", "value"], 2147483648)
count = 0
while true {
    batch = batch_next(reader)
    if len(batch) == 0 { break }
    count = count + stream_write(writer, batch)
    batch = []
}
batch_close(reader)
stream_close(writer)
seal count
''')
            rss = measured([str(engine), str(source)], directory, "run")
            run_dir = next((directory / "runs").iterdir())
            receipt = json.loads((run_dir / "receipt.json").read_text())
            measured([str(engine), "verify", str(run_dir)], directory, "verify")
            summary = receipt["batches"]["readers"][0]
            assert receipt["status"] == "PASS" and summary["rows"] == rows
            assert summary["peak_batch_bytes"] <= summary["batch_limit_bytes"]
            assert digest(directory / "data.csv") == digest(run_dir / "outputs/copy.csv")
            results.append({"input_bytes": (directory / "data.csv").stat().st_size,
                            "rows": rows, "peak_rss_bytes": rss,
                            "peak_batch_bytes": summary["peak_batch_bytes"],
                            "input_sha256": receipt["data_imports"][0]["sha256"],
                            "output_sha256": receipt["generated_artifacts"][0]["sha256"],
                            "receipt_core_sha256": receipt["receipt_core_sha256"],
                            "output_equals_input": True, "verification": "PASS"})
            print(json.dumps(results[-1]), flush=True)
    peaks = [r["peak_rss_bytes"] for r in results]
    assert max(peaks) < 300 * 1024**2, "Measured child RSS exceeds regression ceiling"
    assert max(peaks) - min(peaks) < 64 * 1024**2, "Memory grows materially with input size"
    report = {"schema": "goblin.batch-memory-check.v1", "engine": version,
              "engine_sha256": digest(engine),
              "platform": sys.platform, "results": results,
              "gate": "PASS", "scope": "Measured serial copy; not a process-wide RAM guarantee"}
    with args.report.open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")


if __name__ == "__main__":
    main()
