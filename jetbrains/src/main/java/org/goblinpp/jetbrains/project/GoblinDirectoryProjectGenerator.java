package org.goblinpp.jetbrains.project;

import com.intellij.openapi.application.ApplicationManager;
import com.intellij.openapi.application.WriteAction;
import com.intellij.openapi.fileEditor.FileEditorManager;
import com.intellij.openapi.module.Module;
import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VfsUtil;
import com.intellij.openapi.vfs.VirtualFile;
import com.intellij.platform.DirectoryProjectGeneratorBase;
import com.intellij.platform.ProjectGeneratorPeer;
import org.goblinpp.jetbrains.GoblinIcons;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

import javax.swing.Icon;
import java.io.IOException;
import java.io.UncheckedIOException;

public final class GoblinDirectoryProjectGenerator
        extends DirectoryProjectGeneratorBase<GoblinProjectSettings> {
    @Override
    public @NotNull String getName() {
        return "Goblin++";
    }

    @Override
    public @NotNull Icon getLogo() {
        return GoblinIcons.FILE;
    }

    @Override
    public @NotNull ProjectGeneratorPeer<GoblinProjectSettings> createPeer() {
        return new GoblinProjectGeneratorPeer();
    }

    @Override
    public @Nullable String getDescription() {
        return "Create a Goblin++ scientific programming project.";
    }

    @Override
    public void generateProject(
            @NotNull Project project,
            @NotNull VirtualFile baseDirectory,
            @NotNull GoblinProjectSettings settings,
            @Nullable Module module) {
        WriteAction.runAndWait(() -> {
            try {
                createIfMissing(baseDirectory, "main.gbl", GoblinProjectTemplate.mainProgram(settings.paranoidMode()));
                createIfMissing(baseDirectory, ".gitignore", GoblinProjectTemplate.gitignore());
                createIfMissing(baseDirectory, "README.md", GoblinProjectTemplate.readme(settings.paranoidMode()));
            } catch (IOException error) {
                throw new UncheckedIOException("Unable to create the Goblin++ starter project", error);
            }
        });

        VirtualFile starter = baseDirectory.findChild("main.gbl");
        if (starter != null) {
            ApplicationManager.getApplication().invokeLater(
                    () -> FileEditorManager.getInstance(project).openFile(starter, true));
        }
    }

    private static void createIfMissing(VirtualFile directory, String name, String content)
            throws IOException {
        VirtualFile file = directory.findChild(name);
        if (file == null) {
            file = directory.createChildData(GoblinDirectoryProjectGenerator.class, name);
            VfsUtil.saveText(file, content);
        }
    }
}
