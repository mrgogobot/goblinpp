package org.goblinpp.jetbrains;

import com.intellij.openapi.application.ApplicationManager;
import com.intellij.openapi.components.PersistentStateComponent;
import com.intellij.openapi.components.Service;
import com.intellij.openapi.components.State;
import com.intellij.openapi.components.Storage;
import com.intellij.util.xmlb.XmlSerializerUtil;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

@Service(Service.Level.APP)
@State(name = "GoblinSettings", storages = @Storage("goblinpp.xml"))
public final class GoblinSettingsState implements PersistentStateComponent<GoblinSettingsState> {
    public String executablePath = "";

    public static GoblinSettingsState getInstance() {
        return ApplicationManager.getApplication().getService(GoblinSettingsState.class);
    }

    @Override
    public @Nullable GoblinSettingsState getState() {
        return this;
    }

    @Override
    public void loadState(@NotNull GoblinSettingsState state) {
        XmlSerializerUtil.copyBean(state, this);
    }
}
