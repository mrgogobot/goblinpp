package org.goblinpp.jetbrains.run;

import com.intellij.execution.configurations.ConfigurationFactory;
import com.intellij.execution.configurations.ConfigurationType;
import com.intellij.execution.configurations.RunConfiguration;
import com.intellij.execution.configurations.RunConfigurationOptions;
import com.intellij.openapi.project.Project;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

public final class GoblinRunConfigurationFactory extends ConfigurationFactory {
    GoblinRunConfigurationFactory(@NotNull ConfigurationType type) {
        super(type);
    }

    @Override
    public @NotNull String getId() {
        return GoblinRunConfigurationType.ID;
    }

    @Override
    public @NotNull RunConfiguration createTemplateConfiguration(@NotNull Project project) {
        return new GoblinRunConfiguration(project, this, "Goblin++");
    }

    @Override
    public @Nullable Class<? extends RunConfigurationOptions> getOptionsClass() {
        return GoblinRunConfigurationOptions.class;
    }

    @Override
    public boolean isEditableInDumbMode() {
        return true;
    }
}
