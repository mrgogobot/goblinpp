package org.goblinpp.jetbrains.run;

import org.junit.jupiter.api.Test;

import static org.junit.jupiter.api.Assertions.assertEquals;

import java.util.List;

final class GoblinRunConfigurationOptionsTest {
    @Test
    void defaultsAreEmptyAndValuesRoundTrip() {
        GoblinRunConfigurationOptions options = new GoblinRunConfigurationOptions();
        assertEquals("", options.getScriptPath());
        assertEquals("", options.getProgramArguments());
        assertEquals("", options.getWorkingDirectory());

        options.setScriptPath("examples/energy.gbl");
        options.setProgramArguments("--name Ada");
        options.setWorkingDirectory("examples");

        assertEquals("examples/energy.gbl", options.getScriptPath());
        assertEquals("--name Ada", options.getProgramArguments());
        assertEquals("examples", options.getWorkingDirectory());
    }

    @Test
    void commandPreservesQuotedProgramArgumentsBehindCliBoundary() {
        assertEquals(
                List.of(
                        "goblin++",
                        "run",
                        "/science/my program.gbl",
                        "--",
                        "--name",
                        "Ada Lovelace"),
                GoblinRunCommand.build(
                        "goblin++",
                        "/science/my program.gbl",
                        "--name \"Ada Lovelace\""));
    }

    @Test
    void commandOmitsBoundaryWhenThereAreNoProgramArguments() {
        assertEquals(
                List.of("goblin++", "run", "main.gbl"),
                GoblinRunCommand.build("goblin++", "main.gbl", ""));
    }
}
