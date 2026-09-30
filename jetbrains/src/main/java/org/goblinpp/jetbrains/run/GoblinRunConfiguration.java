package org.goblinpp.jetbrains.run;

import com.intellij.execution.ExecutionException;
import com.intellij.execution.Executor;
import com.intellij.execution.configurations.CommandLineState;
import com.intellij.execution.configurations.ConfigurationFactory;
import com.intellij.execution.configurations.GeneralCommandLine;
import com.intellij.execution.configurations.LocatableConfigurationBase;
import com.intellij.execution.configurations.RunProfileState;
import com.intellij.execution.configurations.RuntimeConfigurationError;
import com.intellij.execution.process.OSProcessHandler;
import com.intellij.execution.process.ProcessHandler;
import com.intellij.execution.process.ProcessHandlerFactory;
import com.intellij.execution.process.ProcessTerminatedListener;
import com.intellij.execution.runners.ExecutionEnvironment;
import com.intellij.openapi.fileEditor.FileDocumentManager;
import com.intellij.openapi.options.SettingsEditor;
import com.intellij.openapi.project.Project;
import org.goblinpp.jetbrains.actions.GoblinCliRunner;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import java.nio.charset.StandardCharsets;
import java.nio.file.Files;
import java.nio.file.Path;
import java.util.List;
import java.util.Locale;

public final class GoblinRunConfiguration
        extends LocatableConfigurationBase<GoblinRunConfigurationOptions> {
    GoblinRunConfiguration(
            @NotNull Project project,
            @NotNull ConfigurationFactory factory,
            @NotNull String name) {
        super(project, factory, name);
    }

    @Override
    protected @NotNull GoblinRunConfigurationOptions getOptions() {
        return (GoblinRunConfigurationOptions) super.getOptions();
    }

    public @NotNull String getScriptPath() {
        return getOptions().getScriptPath();
    }

    public void setScriptPath(@NotNull String value) {
        getOptions().setScriptPath(value);
    }

    public @NotNull String getProgramArguments() {
        return getOptions().getProgramArguments();
    }

    public void setProgramArguments(@NotNull String value) {
        getOptions().setProgramArguments(value);
    }

    public @NotNull String getWorkingDirectory() {
        return getOptions().getWorkingDirectory();
    }

    public void setWorkingDirectory(@NotNull String value) {
        getOptions().setWorkingDirectory(value);
    }

    @Override
    public @Nullable String suggestedName() {
        String script = getScriptPath();
        if (script.isBlank()) {
            return null;
        }
        Path file = Path.of(script);
        Path name = file.getFileName();
        return name == null ? "Goblin++" : name.toString();
    }

    @Override
    public @NotNull SettingsEditor<? extends GoblinRunConfiguration> getConfigurationEditor() {
        return new GoblinRunSettingsEditor(getProject());
    }

    @Override
    public void checkConfiguration() throws RuntimeConfigurationError {
        String script = getScriptPath();
        if (script.isBlank()) {
            throw new RuntimeConfigurationError("Select a Goblin++ program to run.");
        }
        Path source;
        try {
            source = Path.of(script);
        } catch (RuntimeException error) {
            throw new RuntimeConfigurationError("The Goblin++ program path is invalid.");
        }
        if (!Files.isRegularFile(source)) {
            throw new RuntimeConfigurationError("The Goblin++ program does not exist: " + script);
        }
        if (!script.toLowerCase(Locale.ROOT).endsWith(".gbl")) {
            throw new RuntimeConfigurationError("Goblin++ programs must use the .gbl extension.");
        }
        String directory = getWorkingDirectory();
        try {
            if (!directory.isBlank() && !Files.isDirectory(Path.of(directory))) {
                throw new RuntimeConfigurationError(
                        "The working directory does not exist: " + directory);
            }
        } catch (RuntimeConfigurationError error) {
            throw error;
        } catch (RuntimeException error) {
            throw new RuntimeConfigurationError("The working directory path is invalid.");
        }
    }

    @Override
    public @Nullable RunProfileState getState(
            @NotNull Executor executor,
            @NotNull ExecutionEnvironment environment) {
        return new CommandLineState(environment) {
            @Override
            protected @NotNull ProcessHandler startProcess() throws ExecutionException {
                FileDocumentManager.getInstance().saveAllDocuments();

                List<String> command = GoblinRunCommand.build(
                        GoblinCliRunner.executable(getProject()),
                        getScriptPath(),
                        getProgramArguments());

                GeneralCommandLine commandLine = new GeneralCommandLine(command)
                        .withCharset(StandardCharsets.UTF_8);
                String directory = getWorkingDirectory();
                if (!directory.isBlank()) {
                    commandLine.setWorkDirectory(directory);
                }

                OSProcessHandler handler = ProcessHandlerFactory.getInstance()
                        .createColoredProcessHandler(commandLine);
                ProcessTerminatedListener.attach(handler);
                return handler;
            }
        };
    }
}
