package org.goblinpp.jetbrains;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

final class GoblinVocabularyTest {
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
