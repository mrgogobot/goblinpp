package org.goblinpp.jetbrains.actions;

import com.intellij.execution.configurations.GeneralCommandLine;
import com.intellij.execution.executors.DefaultRunExecutor;
import com.intellij.execution.filters.TextConsoleBuilderFactory;
import com.intellij.execution.process.OSProcessHandler;
import com.intellij.execution.ui.RunContentDescriptor;
import com.intellij.execution.ui.RunContentManager;
import com.intellij.openapi.actionSystem.AnAction;
import com.intellij.openapi.actionSystem.AnActionEvent;
import com.intellij.openapi.fileChooser.FileChooser;
import com.intellij.openapi.fileChooser.FileChooserDescriptor;
import com.intellij.openapi.project.DumbAware;
import com.intellij.openapi.project.Project;
import com.intellij.openapi.ui.Messages;
import com.intellij.openapi.vfs.VirtualFile;
import org.jetbrains.annotations.NotNull;
import java.nio.charset.StandardCharsets;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;

/** Confidential operations never use a terminal, run configuration, or scientific receipts. */
public abstract class GoblinConfidentialAction extends AnAction implements DumbAware {
    protected abstract String mode();
    @Override public void update(@NotNull AnActionEvent event) {
        event.getPresentation().setEnabledAndVisible(event.getProject() != null && com.intellij.ide.impl.TrustedProjects.isTrusted(event.getProject()));
    }
    @Override public void actionPerformed(@NotNull AnActionEvent event) {
        Project project = event.getProject();
        if (project == null) return;
        if (!com.intellij.ide.impl.TrustedProjects.isTrusted(project)) {
            Messages.showErrorDialog(project, "File protection is disabled in an untrusted project. Use the IDE project-trust controls before entering secrets.", "Goblin++"); return;
        }
        boolean reading = mode().equals("read-file");
        boolean textMode = mode().equals("encrypt-text");
        boolean encrypt = mode().startsWith("encrypt");
        String text = null;
        VirtualFile input = null;
        if (textMode) {
            text = Messages.showPasswordDialog(project, "Text to encrypt (hidden; never stored in source):", "Goblin++ Encrypt Text", Messages.getQuestionIcon());
            if (text == null) return;
            if (text.getBytes(StandardCharsets.UTF_8).length > 1048576) { Messages.showErrorDialog(project, "New text entry is limited to 1 MiB; encrypt a file instead.", "Goblin++"); return; }
        } else {
            FileChooserDescriptor chooser = new FileChooserDescriptor(true, false, false, false, false, false);
            chooser.setTitle(reading ? "Display a UTF-8 File" : "Select Input File");
            input = FileChooser.chooseFile(chooser, project, null);
            if (input == null || !input.isInLocalFileSystem()) return;
        }
        List<String> args = new ArrayList<>();
        String password = null;
        if (reading) {
            if (Messages.showYesNoDialog(project, "Display plaintext in the Run console? It may remain in IDE logs and screen history.", "Goblin++ Text Preview", Messages.getWarningIcon()) != Messages.YES) return;
            args.addAll(List.of("read-file", input.getPath()));
        } else {
            String output = Messages.showInputDialog(project, "Absolute path for a NEW output file (existing files are refused):", "Goblin++ Output File", Messages.getQuestionIcon());
            if (output == null) return;
            try {
                if (!java.nio.file.Path.of(output).isAbsolute()) {
                    Messages.showErrorDialog(project, "Use an absolute output path.", "Goblin++"); return;
                }
            } catch (java.nio.file.InvalidPathException error) {
                Messages.showErrorDialog(project, "Invalid output path.", "Goblin++"); return;
            }
            password = Messages.showPasswordDialog(project, "Passphrase (hidden; encryption requires at least 12 UTF-8 bytes):", "Goblin++ File Protection", Messages.getQuestionIcon());
            if (password == null) return;
            int size = password.getBytes(StandardCharsets.UTF_8).length;
            if (size == 0 || size > 4096 || password.indexOf('\n') >= 0 || password.indexOf('\r') >= 0 || password.indexOf('\0') >= 0 || (encrypt && size < 12)) {
                Messages.showErrorDialog(project, "Invalid passphrase length or newline/NUL.", "Goblin++"); return;
            }
            if (encrypt) {
                String confirm = Messages.showPasswordDialog(project, "Confirm encryption passphrase:", "Goblin++ File Protection", Messages.getQuestionIcon());
                if (confirm == null) return;
                if (!confirm.equals(password)) { Messages.showErrorDialog(project, "Passphrases did not match.", "Goblin++"); return; }
            }
            args.addAll(List.of(encrypt ? "encrypt" : "decrypt", "--key-stdin", "--output", output));
            if (textMode) args.add("--text-stdin");
            else args.addAll(List.of("--input", input.getPath()));
        }
        List<String> command = new ArrayList<>();
        command.add(GoblinCliRunner.executable(project));
        command.addAll(args);
        byte[] payload = reading ? new byte[0] : (password + "\n" + (text == null ? "" : text)).getBytes(StandardCharsets.UTF_8);
        // Java/IDE String objects cannot promise secure erasure; never persist them in settings.
        try {
            GeneralCommandLine line = new GeneralCommandLine(command).withCharset(StandardCharsets.UTF_8);
            if (project.getBasePath() != null) line.setWorkDirectory(project.getBasePath());
            OSProcessHandler handler = new OSProcessHandler(line);
            var console = TextConsoleBuilderFactory.getInstance().createBuilder(project).getConsole();
            console.attachToProcess(handler);
            RunContentManager.getInstance(project).showRunContent(DefaultRunExecutor.getRunExecutorInstance(),
                    new RunContentDescriptor(console, handler, console.getComponent(), "Goblin++ File Protection"));
            handler.startNotify();
            // Write directly to the child's pipe; do not use console input (which can echo).
            var pipe = handler.getProcessInput();
            com.intellij.openapi.application.ApplicationManager.getApplication().executeOnPooledThread(() -> {
                try { if (pipe != null) { pipe.write(payload); pipe.close(); } }
                catch (java.io.IOException ignored) { /* Child reports redacted failure in console. */ }
                finally { Arrays.fill(payload, (byte) 0); }
            });
        } catch (Exception error) {
            Arrays.fill(payload, (byte) 0);
            Messages.showErrorDialog(project, "The file operation could not start. Check the executable and output path. Secret values are omitted.", "Goblin++");
        }
    }
}
