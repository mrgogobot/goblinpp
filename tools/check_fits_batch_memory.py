#!/usr/bin/env python3
"""Opt-in real FITS/RSS stress gate; several GiB of free temporary disk required.

Only synthetic files in a dedicated temporary directory are used. This measures
serial projection/export and verification, not a general process-memory quota.
The wide, unselected ASCII column makes RAM growth visible without huge row loops.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import sys
import tempfile

from check_batch_memory import digest, measured


def header(cards):
    payload = b"".join(f"{key:<8}= {value}".ljust(80).encode("ascii") for key, value in cards)
    payload += b"END".ljust(80)
    return payload.ljust((len(payload) + 2879) // 2880 * 2880, b" ")


def write_fixture(path, rows):
    primary = header([("SIMPLE", "T"), ("BITPIX", "8"), ("NAXIS", "0"), ("EXTEND", "T")])
    table = header([("XTENSION", "'BINTABLE'"), ("BITPIX", "8"), ("NAXIS", "2"),
                    ("NAXIS1", "4000"), ("NAXIS2", str(rows)), ("PCOUNT", "0"),
                    ("GCOUNT", "1"), ("TFIELDS", "3"), ("TTYPE1", "'SOURCE_ID'"),
                    ("TFORM1", "'1K'"), ("TTYPE2", "'VALUE'"), ("TFORM2", "'1D'"),
                    ("TUNIT2", "'m'"), ("TTYPE3", "'UNSELECTED'"), ("TFORM3", "'3984A'")])
    row = struct.pack(">qd", 9007199254740993, 2.5) + b"x" * 3984
    with path.open("xb") as stream:
        stream.write(primary + table)
        block = row * 1024
        for _ in range(rows // 1024):
            stream.write(block)
        stream.write(row * (rows % 1024))
        stream.write(b"\0" * (-rows * len(row) % 2880))


def expected_csv_hash(rows):
    h = hashlib.sha256(b"SOURCE_ID,VALUE\n")
    row = b"9007199254740993,2.5\n"
    for _ in range(rows // 4096):
        h.update(row * 4096)
    h.update(row * (rows % 4096))
    return h.hexdigest()


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("engine", type=Path)
    parser.add_argument("--report", required=True, type=Path)
    parser.add_argument("--large", action="store_true", help="Include a real file larger than 1 GiB")
    args = parser.parse_args()
    if args.report.exists():
        raise SystemExit("Refusing to overwrite a measured report")
    engine = args.engine.resolve()
    version = subprocess.check_output([str(engine), "--version"], text=True).strip()
    targets = [2 * 1024**2, 64 * 1024**2] + ([1024**3 + 2 * 1024**2] if args.large else [])
    results = []
    with tempfile.TemporaryDirectory(prefix="goblin-fits-batch-stress-") as temporary:
        root = Path(temporary)
        for size in targets:
            directory = root / str(size)
            directory.mkdir()
            rows = size // 4000 + 1
            path = directory / "data.fits"
            write_fixture(path, rows)
            source = directory / "stress.gbl"
            source.write_text('''GO_PARANOID
reader = fits_batch_open("data.fits", 1, ["SOURCE_ID", "VALUE"], 4096, 8388608, 2147483648)
writer = csv_stream_open("copy.csv", ["SOURCE_ID", "VALUE"], 2147483648)
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
            run = next((directory / "runs").iterdir())
            receipt = json.loads((run / "receipt.json").read_text())
            verify_rss = measured([str(engine), "verify", str(run)], directory, "verify")
            summary = receipt["batches"]["readers"][0]
            assert receipt["status"] == "PASS" and summary["rows"] == rows
            assert summary["fits"]["raw_row_bytes"] == 4000
            assert summary["fits"]["columns"][1]["unit"] == "m"
            assert summary["closed"] and summary["eof"]
            assert summary["peak_batch_bytes"] <= summary["batch_limit_bytes"]
            assert digest(run / "outputs/copy.csv") == expected_csv_hash(rows)
            assert digest(path) == receipt["data_imports"][0]["sha256"]
            results.append({"input_bytes": path.stat().st_size, "rows": rows,
                            "peak_rss_bytes": rss, "verification_peak_rss_bytes": verify_rss,
                            "peak_batch_bytes": summary["peak_batch_bytes"],
                            "source_sha256": receipt["source"]["sha256"],
                            "input_sha256": receipt["data_imports"][0]["sha256"],
                            "output_sha256": receipt["generated_artifacts"][0]["sha256"],
                            "receipt_core_sha256": receipt["receipt_core_sha256"],
                            "exact_ids_and_rows": True, "verification": "PASS"})
            print(json.dumps(results[-1]), flush=True)
    peaks = [r["peak_rss_bytes"] for r in results]
    assert max(peaks) < 300 * 1024**2, "Child RSS exceeds the measured regression ceiling"
    assert max(peaks) - min(peaks) < 64 * 1024**2, "Memory grows materially with input size"
    assert max(r["verification_peak_rss_bytes"] for r in results) < 300 * 1024**2
    report = {"schema": "goblin.fits-batch-memory-check.v1", "engine": version,
              "engine_sha256": digest(engine), "platform": sys.platform,
              "results": results, "gate": "PASS",
              "scope": "Measured synthetic serial FITS projection/export; not a process-wide RAM guarantee"}
    with args.report.open("x") as stream:
        json.dump(report, stream, indent=2)
        stream.write("\n")


if __name__ == "__main__":
    main()
