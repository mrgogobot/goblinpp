package org.goblinpp.jetbrains.project;

public final class GoblinProjectTemplate {
    public static String mainProgram(boolean paranoidMode) {
        String directive = paranoidMode ? "GO_PARANOID\n\n" : "";
        return directive
                + "samples = 12\n"
                + "accepted = 9\n\n"
                + "fraction = accepted / samples\n\n"
                + "print(\"accepted fraction = {fraction:.3f}\")\n"
                + "seal fraction\n";
    }

    public static String gitignore() {
        return "/.goblin/\n"
                + "**/runs/\n"
                + "*.freeze.json\n"
                + "*.lineage.json\n"
                + "*.goblin.rs\n"
                + "/target/\n"
                + "/build/\n"
                + ".env\n"
                + ".env.*\n";
    }

    public static String readme(boolean paranoidMode) {
        String mode = paranoidMode
                ? "This starter enables `GO_PARANOID` for postflight source and receipt checks."
                : "This starter uses everyday mode. Add `GO_PARANOID` as the first statement when audit postflight checks are required.";
        return "# Goblin++ project\n\n"
                + mode + "\n\n"
                + "Run the starter:\n\n"
                + "```console\n"
                + "goblin++ main.gbl\n"
                + "```\n\n"
                + "Useful checks:\n\n"
                + "```console\n"
                + "goblin++ check main.gbl\n"
                + "goblin++ freeze main.gbl\n"
                + "goblin++ status main.gbl\n"
                + "```\n\n"
                + "The JetBrains plugin is an editor integration; the native Goblin++ executable must be installed separately.\n";
    }

    private GoblinProjectTemplate() {
    }
}
