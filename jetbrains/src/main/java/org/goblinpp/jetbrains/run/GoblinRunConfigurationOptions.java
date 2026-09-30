package org.goblinpp.jetbrains.run;

import com.intellij.execution.configurations.LocatableRunConfigurationOptions;
import com.intellij.openapi.components.StoredProperty;
import org.jetbrains.annotations.NotNull;

public final class GoblinRunConfigurationOptions extends LocatableRunConfigurationOptions {
    private final StoredProperty<String> scriptPath = string("").provideDelegate(this, "scriptPath");
    private final StoredProperty<String> programArguments = string("").provideDelegate(this, "programArguments");
    private final StoredProperty<String> workingDirectory = string("").provideDelegate(this, "workingDirectory");

    public @NotNull String getScriptPath() {
        return scriptPath.getValue(this);
    }

    public void setScriptPath(@NotNull String value) {
        scriptPath.setValue(this, value);
    }

    public @NotNull String getProgramArguments() {
        return programArguments.getValue(this);
    }

    public void setProgramArguments(@NotNull String value) {
        programArguments.setValue(this, value);
    }

    public @NotNull String getWorkingDirectory() {
        return workingDirectory.getValue(this);
    }

    public void setWorkingDirectory(@NotNull String value) {
        workingDirectory.setValue(this, value);
    }
}
