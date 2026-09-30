package org.goblinpp.jetbrains.run;

import com.intellij.openapi.fileChooser.FileChooserDescriptorFactory;
import com.intellij.openapi.options.ConfigurationException;
import com.intellij.openapi.options.SettingsEditor;
import com.intellij.openapi.project.Project;
import com.intellij.openapi.ui.TextFieldWithBrowseButton;
import com.intellij.ui.components.JBTextField;
import com.intellij.util.ui.FormBuilder;
import org.jetbrains.annotations.NotNull;

import javax.swing.JComponent;
import javax.swing.JPanel;

final class GoblinRunSettingsEditor extends SettingsEditor<GoblinRunConfiguration> {
    private final TextFieldWithBrowseButton scriptPath = new TextFieldWithBrowseButton();
    private final JBTextField programArguments = new JBTextField();
    private final TextFieldWithBrowseButton workingDirectory = new TextFieldWithBrowseButton();
    private final JPanel panel;

    GoblinRunSettingsEditor(@NotNull Project project) {
        scriptPath.addBrowseFolderListener(
                project,
                FileChooserDescriptorFactory.createSingleFileDescriptor("gbl")
                        .withTitle("Select Goblin++ Program"));
        workingDirectory.addBrowseFolderListener(
                project,
                FileChooserDescriptorFactory.createSingleFolderDescriptor()
                        .withTitle("Select Working Directory"));
        panel = FormBuilder.createFormBuilder()
                .addLabeledComponent("Program:", scriptPath)
                .addLabeledComponent("Program arguments:", programArguments)
                .addLabeledComponent("Working directory:", workingDirectory)
                .addComponentFillVertically(new JPanel(), 0)
                .getPanel();
    }

    @Override
    protected void resetEditorFrom(@NotNull GoblinRunConfiguration configuration) {
        scriptPath.setText(configuration.getScriptPath());
        programArguments.setText(configuration.getProgramArguments());
        workingDirectory.setText(configuration.getWorkingDirectory());
    }

    @Override
    protected void applyEditorTo(@NotNull GoblinRunConfiguration configuration)
            throws ConfigurationException {
        configuration.setScriptPath(scriptPath.getText().trim());
        configuration.setProgramArguments(programArguments.getText());
        configuration.setWorkingDirectory(workingDirectory.getText().trim());
    }

    @Override
    protected @NotNull JComponent createEditor() {
        return panel;
    }
}
