# Read-only Goblin++ check

The extension calls `goblin++ check FILE --json` against saved source. This is a machinery preview, not evidence:

```text
AUTHORITY=PREVIEW_ONLY_NOT_EVIDENCE
EVIDENCE_CREATED=NO
CUSTODY_CHECKED=NO
```

For a parsed program, the Rust CLI returns `goblin.check.v1`. It reports `status`, a single nullable `diagnostic`, starting source and canonical hashes, and previews of stdout, explicitly sealed values, and generated outputs. The extension checks that the report does not claim evidence or custody authority. A failed evaluation appears as a document-level diagnostic because v1 does not provide a precise source range.

Lexical and parsing failures currently exit before the CLI can emit JSON. The extension recognizes their `G001`/`G002` stderr messages and displays a document-level preview diagnostic. Other command failures remain command failures, not source diagnostics. The extension never invents a line number.

Check may read a referenced FITS input or render an output preview in memory. It does not write run directories, receipts, output files, or ledger events. It does not check frozen-source authorization. **Check on Save** is opt-in, and stale results from older checks are ignored.

Use **Run Current File**, then **Verify Run Directory**, before treating results as preserved evidence. A passing check is not a claim that a model, data selection, unit interpretation, or conclusion is scientifically sound.
