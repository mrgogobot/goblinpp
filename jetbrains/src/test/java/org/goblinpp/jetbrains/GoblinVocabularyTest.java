package org.goblinpp.jetbrains;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotNull;
import static org.junit.jupiter.api.Assertions.assertTrue;

final class GoblinVocabularyTest {
    @Test
    void loadsCurrentEditorVocabulary() {
        assertTrue(GoblinVocabulary.entries().size() >= 100);
        assertEquals("function", GoblinVocabulary.find("sqrt").kind());
        assertEquals("function", GoblinVocabulary.find("cross").kind());
        assertEquals("function", GoblinVocabulary.find("chem_molar_mass").kind());
        assertEquals("function", GoblinVocabulary.find("ee_current").kind());
        assertEquals("function", GoblinVocabulary.find("ee_kcl_balanced").kind());
        assertEquals("function", GoblinVocabulary.find("ee_in_unit").kind());
        assertEquals("constant", GoblinVocabulary.find("ħ").kind());
        assertEquals("constant", GoblinVocabulary.find("m_u").kind());
        assertEquals("unit", GoblinVocabulary.find("kg").kind());
        assertEquals("unit", GoblinVocabulary.find("µL").kind());
        assertEquals("unit", GoblinVocabulary.find("Å").kind());
        assertEquals("unit", GoblinVocabulary.find("Ω").kind());
        assertEquals("unit", GoblinVocabulary.find("µF").kind());
        assertEquals("unit", GoblinVocabulary.find("A").kind());
        assertNotNull(GoblinVocabulary.find("GO_PARANOID"));
    }
}
