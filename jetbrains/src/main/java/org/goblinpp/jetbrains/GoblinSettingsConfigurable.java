package org.goblinpp.jetbrains;

import com.intellij.openapi.options.Configurable;
import com.intellij.ui.components.JBLabel;
import com.intellij.ui.components.JBTextField;
import com.intellij.util.ui.FormBuilder;
import org.jetbrains.annotations.Nls;
import org.jetbrains.annotations.Nullable;

import javax.swing.JComponent;
import javax.swing.JPanel;

public final class GoblinSettingsConfigurable implements Configurable {
    private JBTextField executablePath;
    private JPanel panel;

    @Override
    public @Nls String getDisplayName() {
        return "Goblin++";
    }

    @Override
    public @Nullable JComponent createComponent() {
        executablePath = new JBTextField();
        panel = FormBuilder.createFormBuilder()
                .addLabeledComponent(new JBLabel("Goblin++ executable:"), executablePath, 1, false)
                .addComponentFillVertically(new JPanel(), 0)
                .getPanel();
        reset();
        return panel;
    }

    @Override
    public boolean isModified() {
        return executablePath != null
                && !executablePath.getText().trim().equals(GoblinSettingsState.getInstance().executablePath);
    }

    @Override
    public void apply() {
        if (executablePath != null) {
            GoblinSettingsState.getInstance().executablePath = executablePath.getText().trim();
        }
    }

    @Override
    public void reset() {
        if (executablePath != null) {
            executablePath.setText(GoblinSettingsState.getInstance().executablePath);
        }
    }

    @Override
    public void disposeUIResources() {
        executablePath = null;
        panel = null;
    }
}
