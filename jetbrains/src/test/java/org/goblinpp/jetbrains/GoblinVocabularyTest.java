package org.goblinpp.jetbrains;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

final class GoblinVocabularyTest {
    @Test
    void textAndFluidCompletionKeepsCriticalContractsVisible() {
        for (String name : new String[]{"str_upper", "str_lower", "str_bytes_len", "sizeof", "str_compare", "str_to_hex", "str_from_hex", "str_to_octal", "str_from_octal", "str_to_binary", "str_from_binary", "to_base", "from_base", "fluid_density", "fluid_kinematic_viscosity", "fluid_dynamic_viscosity", "fluid_volume_flow", "fluid_flow_speed", "fluid_reynolds", "fluid_dynamic_pressure", "fluid_bernoulli_total_pressure", "fluid_pipe_pressure_drop"}) {
            var entry = GoblinVocabulary.find(name);
            assertNotNull(entry, name);
            assertEquals("function", entry.kind());
            assertTrue(entry.snippet().startsWith(name + "("));
        }
        assertTrue(GoblinVocabulary.find("sizeof").detail().contains("Not C object size"));
        assertTrue(GoblinVocabulary.find("fluid_pipe_pressure_drop").detail().contains("Re < 2000"));
        assertTrue(GoblinVocabulary.find("fluid_pipe_pressure_drop").detail().contains("Radius, not diameter"));
    }

    @Test
    void arrayMathHelpStatesShapeCopyAndDiagnostics() {
        for (String name : new String[]{"sqrt", "min", "hypot", "sind", "atan2r"}) {
            var detail = GoblinVocabulary.find(name).detail();
            assertTrue(detail.contains("element-wise"));
            assertTrue(detail.contains("zero-based"));
            assertTrue(detail.contains("No reduction"));
        }
        for (String name : new String[]{"dot", "matrix_multiply", "is_close", "same_bits"}) {
            assertFalse(GoblinVocabulary.find(name).detail().contains("Also element-wise"));
        }
    }
    @Test
    void arrayPlotCompletionDocumentsScientificEvidence() {
        for (String name : new String[]{"plot_line", "plot_scatter", "plot_histogram"}) {
            var entry = GoblinVocabulary.find(name);
            assertEquals("function", entry.kind());
            assertTrue(entry.snippet().contains(name));
            assertTrue(entry.detail().contains("sidecar"));
        }
        assertTrue(GoblinVocabulary.find("plot_line").detail().contains("supplied order"));
        assertTrue(GoblinVocabulary.find("plot_histogram").detail().contains("2..500"));
    }
    @Test
    void clockCompletionDocumentsCapturedTextAndUtc() {
        var entry = GoblinVocabulary.find("time_date");
        assertEquals("function", entry.kind());
        assertTrue(entry.snippet().contains("utc"));
        assertTrue(entry.detail().contains("DD-MM-YYYY:HH:MM:SS"));
        assertTrue(entry.detail().contains("not certified time"));
    }
    @Test
    void exactIdJoinsHaveCompletionAndExplicitRules() {
        for (String name : new String[]{"csv_join", "tsv_join"}) {
            var entry = GoblinVocabulary.find(name);
            assertEquals("function", entry.kind());
            assertTrue(entry.snippet().contains(name));
            assertTrue(entry.detail().contains("one-to-one"));
            assertTrue(entry.detail().contains("duplicate"));
            assertTrue(entry.detail().contains("16 MiB"));
        }
    }
    @Test
    void serialBatchesHaveCompletionAndExplicitLimits() {
        for (String name : new String[]{"csv_batch_open", "tsv_batch_open", "batch_next", "batch_close", "csv_stream_open", "tsv_stream_open", "stream_write", "stream_close"}) {
            assertEquals("function", GoblinVocabulary.find(name).kind());
            assertTrue(GoblinVocabulary.find(name).snippet().contains(name));
        }
        assertTrue(GoblinVocabulary.find("csv_batch_open").detail().contains("64 GiB"));
        assertTrue(GoblinVocabulary.find("batch_next").detail().contains("not a process-wide RAM quota"));
        assertTrue(GoblinVocabulary.find("csv_stream_open").detail().contains("incomplete"));
    }
    @Test
    void fitsBatchesExplainExactIdsAndMissingValues() {
        var entry = GoblinVocabulary.find("fits_batch_open");
        assertEquals("function", entry.kind());
        assertTrue(entry.snippet().contains("SOURCE_ID"));
        assertTrue(entry.detail().contains("case-sensitive"));
        assertTrue(entry.detail().contains("\\N"));
        assertTrue(entry.detail().contains("not physical blinding"));
    }
    @Test
    void inferenceHelpersStateTheirScientificChoices() {
        for (String name : new String[]{"resample", "bootstrap_mean", "bootstrap_median", "rng_normal", "rng_normal_array", "matrix_transpose", "matrix_multiply", "vector_norm", "vector_unit", "covariance", "cholesky", "mvnormal", "csv_scan_stats", "tsv_scan_stats"}) {
            assertEquals("function", GoblinVocabulary.find(name).kind());
            assertTrue(GoblinVocabulary.find(name).snippet().contains(name));
        }
        assertTrue(GoblinVocabulary.find("covariance").detail().contains("ddof"));
        assertTrue(GoblinVocabulary.find("cholesky").detail().contains("no jitter"));
        assertTrue(GoblinVocabulary.find("GO_LOOP_BUDGET").detail().contains("50,000,000"));
    }
    @Test
    void auditCompletionExplainsExplicitExecutionAndClaimLimits() {
        var entry = GoblinVocabulary.find("GO_NUMERIC_AUDIT");
        assertTrue(entry.detail().contains("run --compile"));
        assertTrue(entry.detail().contains("not an accuracy certificate"));
        assertTrue(entry.snippet().contains("numeric_audit.json"));
    }

    @Test
    void finalStateCompletionExplainsScopeAndTrustLimits() {
        var entry = GoblinVocabulary.find("COMPLETELY_MAD");
        assertTrue(entry.detail().contains("256 bindings and 8 MiB"));
        assertTrue(entry.detail().contains("separately from explicit snapshots"));
        assertTrue(entry.detail().contains("not encryption or scientific validity"));
        assertTrue(entry.snippet().equals("COMPLETELY_MAD"));
    }
    @Test
    void protectedValuesHaveExplicitCompletionHelp() {
        assertTrue(GoblinVocabulary.find("h").detail().contains("Read-only"));
        assertTrue(GoblinVocabulary.find("append").detail().contains("Discarded"));
        assertTrue(GoblinVocabulary.find("input").detail().contains("capture immediately"));
        assertTrue(GoblinVocabulary.find("to_text").detail().contains("round-trips, including -0"));
    }
    @Test
    void loadsCurrentEditorVocabulary() {
        for (String name : new String[]{"rng_seed", "rng_word", "rng_uniform", "rng_integer"}) {
            assertEquals("function", GoblinVocabulary.find(name).kind());
            assertTrue(GoblinVocabulary.find(name).detail().contains("NOT"));
            assertTrue(GoblinVocabulary.find(name).snippet().contains("rng_"));
        }
        for (String name : new String[]{"sort", "median", "quantile", "std_population", "std_sample", "ecdf"}) {
            assertEquals("function", GoblinVocabulary.find(name).kind());
            assertTrue(GoblinVocabulary.find(name).snippet().contains("values"));
        }
        assertTrue(GoblinVocabulary.find("quantile").detail().contains("type 7"));
        assertTrue(GoblinVocabulary.find("std_sample").detail().contains("n-1"));
        assertEquals("function", GoblinVocabulary.find("is_close").kind());
        assertTrue(GoblinVocabulary.find("is_close").detail().contains("no defaults"));
        assertTrue(GoblinVocabulary.find("same_bits").detail().contains("sign of zero"));
        assertTrue(GoblinVocabulary.entries().size() >= 100);
        assertEquals("function", GoblinVocabulary.find("sqrt").kind());
        assertEquals("function", GoblinVocabulary.find("sum").kind());
        assertEquals("statement", GoblinVocabulary.find("import").kind());
        assertEquals("function", GoblinVocabulary.find("csv_numbers").kind());
        assertEquals("function", GoblinVocabulary.find("tsv_column").kind());
        assertTrue(GoblinVocabulary.find("csv_numbers").detail().contains("dimensionless"));
        assertEquals("function", GoblinVocabulary.find("mean").kind());
        assertTrue(GoblinVocabulary.find("sum").detail().contains("non-empty"));
        assertTrue(GoblinVocabulary.find("mean").snippet().contains("values"));
        assertEquals("function", GoblinVocabulary.find("cross").kind());
        for (String name : new String[]{"fits_where", "fits_all", "fits_any", "fits_column_text", "fits_export_csv", "fits_export_tsv"}) {
            assertEquals("function", GoblinVocabulary.find(name).kind());
            assertTrue(!GoblinVocabulary.find(name).snippet().isEmpty());
        }
        assertEquals("function", GoblinVocabulary.find("chem_molar_mass").kind());
        assertEquals("function", GoblinVocabulary.find("ee_current").kind());
        assertEquals("function", GoblinVocabulary.find("ee_kcl_balanced").kind());
        assertEquals("function", GoblinVocabulary.find("ee_in_unit").kind());
        assertEquals("constant", GoblinVocabulary.find("ħ").kind());
        assertEquals("constant", GoblinVocabulary.find("m_u").kind());
        assertEquals("unit", GoblinVocabulary.find("kg").kind());
        assertTrue(GoblinVocabulary.find("m").detail().contains("3 square metres"));
        assertTrue(GoblinVocabulary.find("s").detail().contains("denominator factors"));
        assertEquals("unit", GoblinVocabulary.find("µL").kind());
        assertEquals("unit", GoblinVocabulary.find("Å").kind());
        assertEquals("unit", GoblinVocabulary.find("Ω").kind());
        assertEquals("unit", GoblinVocabulary.find("µF").kind());
        assertEquals("unit", GoblinVocabulary.find("A").kind());
        assertNotNull(GoblinVocabulary.find("GO_PARANOID"));
    }
}
