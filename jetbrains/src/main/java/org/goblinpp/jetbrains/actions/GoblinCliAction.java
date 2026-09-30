package org.goblinpp.jetbrains.actions;

import com.intellij.openapi.actionSystem.AnAction;
import com.intellij.openapi.actionSystem.AnActionEvent;
import com.intellij.openapi.actionSystem.CommonDataKeys;
import com.intellij.openapi.fileEditor.FileDocumentManager;
import com.intellij.openapi.project.DumbAware;
import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VirtualFile;
import org.jetbrains.annotations.NotNull;

import java.util.List;

abstract class GoblinCliAction extends AnAction implements DumbAware {
    protected abstract String title();

    protected abstract List<String> arguments(Project project, VirtualFile file);

    protected boolean requiresGoblinFile() {
        return true;
    }

    @Override
    public void update(@NotNull AnActionEvent event) {
        Project project = event.getProject();
        VirtualFile file = event.getData(CommonDataKeys.VIRTUAL_FILE);
        boolean available = project != null && (!requiresGoblinFile() || isGoblinFile(file));
        event.getPresentation().setEnabledAndVisible(available);
    }

    @Override
    public void actionPerformed(@NotNull AnActionEvent event) {
        Project project = event.getProject();
        VirtualFile file = event.getData(CommonDataKeys.VIRTUAL_FILE);
        if (project == null || (requiresGoblinFile() && !isGoblinFile(file))) {
            return;
        }
        FileDocumentManager.getInstance().saveAllDocuments();
        GoblinCliRunner.run(project, title(), arguments(project, file));
    }

    private static boolean isGoblinFile(VirtualFile file) {
        return file != null && "gbl".equalsIgnoreCase(file.getExtension());
    }
}
