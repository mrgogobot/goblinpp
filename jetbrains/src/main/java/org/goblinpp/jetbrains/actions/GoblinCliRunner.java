package org.goblinpp.jetbrains.actions;

import com.intellij.execution.ExecutionException;
import com.intellij.execution.configurations.GeneralCommandLine;
import com.intellij.execution.executors.DefaultRunExecutor;
import com.intellij.execution.process.OSProcessHandler;
import com.intellij.execution.ui.ConsoleView;
import com.intellij.execution.ui.RunContentDescriptor;
import com.intellij.execution.ui.RunContentManager;
import com.intellij.execution.filters.TextConsoleBuilderFactory;
import com.intellij.notification.NotificationGroupManager;
import com.intellij.notification.NotificationType;
import com.intellij.openapi.project.Project;
import org.goblinpp.jetbrains.GoblinSettingsState;

import java.nio.file.Files;
import java.nio.file.Path;
import java.util.ArrayList;
import java.util.List;

public final class GoblinCliRunner {
    static void run(Project project, String title, List<String> arguments) {
        String executable = executable(project);
        List<String> command = new ArrayList<>();
        command.add(executable);
        command.addAll(arguments);
        GeneralCommandLine commandLine = new GeneralCommandLine(command)
                .withCharset(java.nio.charset.StandardCharsets.UTF_8);
        if (project.getBasePath() != null) {
            commandLine.setWorkDirectory(project.getBasePath());
        }
        try {
            OSProcessHandler handler = new OSProcessHandler(commandLine);
            ConsoleView console = TextConsoleBuilderFactory.getInstance()
                    .createBuilder(project)
                    .getConsole();
            console.attachToProcess(handler);
            RunContentDescriptor descriptor = new RunContentDescriptor(
                    console,
                    handler,
                    console.getComponent(),
                    "Goblin++: " + title);
            RunContentManager.getInstance(project).showRunContent(
                    DefaultRunExecutor.getRunExecutorInstance(),
                    descriptor);
            handler.startNotify();
        } catch (ExecutionException error) {
            NotificationGroupManager.getInstance()
                    .getNotificationGroup("Goblin++")
                    .createNotification(
                            "Goblin++ could not start",
                            error.getMessage() == null ? "Configure the executable path under Tools | Goblin++." : error.getMessage(),
                            NotificationType.ERROR)
                    .notify(project);
        }
    }

    public static String executable(Project project) {
        String configured = GoblinSettingsState.getInstance().executablePath.trim();
        if (!configured.isEmpty()) {
            return configured;
        }
        if (project.getBasePath() != null) {
            Path local = Path.of(project.getBasePath(), "target", "release", "goblin++");
            if (Files.isExecutable(local)) {
                return local.toString();
            }
        }
        String home = System.getProperty("user.home", "");
        if (!home.isBlank()) {
            Path installed = Path.of(home, ".local", "bin", "goblin++");
            if (Files.isExecutable(installed)) {
                return installed.toString();
            }
        }
        return "goblin++";
    }

    private GoblinCliRunner() {
    }
}
