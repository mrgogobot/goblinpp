package org.goblinpp.jetbrains.project;

import com.intellij.ide.util.projectWizard.SettingsStep;
import com.intellij.openapi.ui.ValidationInfo;
import com.intellij.platform.ProjectGeneratorPeer;
import com.intellij.ui.components.JBCheckBox;
import com.intellij.ui.components.JBLabel;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

public final class GoblinProjectGeneratorPeer implements ProjectGeneratorPeer<GoblinProjectSettings> {
    private final JBCheckBox paranoidMode = new JBCheckBox(
            "Include GO_PARANOID audit directive",
            true);

    @Override
    public void buildUI(@NotNull SettingsStep settingsStep) {
        settingsStep.addSettingsComponent(new JBLabel(
                "Creates main.gbl, README.md, and a Goblin++ .gitignore."));
        settingsStep.addSettingsComponent(paranoidMode);
    }

    @Override
    public @NotNull GoblinProjectSettings getSettings() {
        return new GoblinProjectSettings(paranoidMode.isSelected());
    }

    @Override
    public boolean isBackgroundJobRunning() {
        return false;
    }

    @Override
    public @Nullable ValidationInfo validate() {
        return null;
    }
}
