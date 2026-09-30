package org.goblinpp.jetbrains.run;

import com.intellij.execution.configurations.ConfigurationTypeBase;
import com.intellij.openapi.project.DumbAware;
import com.intellij.openapi.util.NotNullLazyValue;
import org.goblinpp.jetbrains.GoblinIcons;

public final class GoblinRunConfigurationType extends ConfigurationTypeBase implements DumbAware {
    public static final String ID = "Goblin++RunConfiguration";

    public GoblinRunConfigurationType() {
        super(
                ID,
                "Goblin++",
                "Run a Goblin++ scientific program",
                NotNullLazyValue.createValue(() -> GoblinIcons.FILE));
        addFactory(new GoblinRunConfigurationFactory(this));
    }
}
