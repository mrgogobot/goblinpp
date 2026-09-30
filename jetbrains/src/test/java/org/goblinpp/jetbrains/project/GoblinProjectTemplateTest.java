package org.goblinpp.jetbrains.project;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertTrue;

final class GoblinProjectTemplateTest {
    @Test
    void createsAuditableStarterWhenRequested() {
        String program = GoblinProjectTemplate.mainProgram(true);
        assertTrue(program.startsWith("GO_PARANOID\n"));
        assertTrue(program.contains("seal fraction"));
        assertTrue(program.contains("{fraction:.3f}"));
    }

    @Test
    void everydayStarterLeavesParanoidModeOptional() {
        String program = GoblinProjectTemplate.mainProgram(false);
        assertFalse(program.contains("GO_PARANOID"));
        assertTrue(GoblinProjectTemplate.readme(false).contains("everyday mode"));
    }

    @Test
    void ignoresGeneratedEvidenceAndBuildOutputs() {
        String ignore = GoblinProjectTemplate.gitignore();
        assertTrue(ignore.contains("/.goblin/"));
        assertTrue(ignore.contains("**/runs/"));
        assertTrue(ignore.contains("*.freeze.json"));
        assertTrue(ignore.contains("/target/"));
    }
}
