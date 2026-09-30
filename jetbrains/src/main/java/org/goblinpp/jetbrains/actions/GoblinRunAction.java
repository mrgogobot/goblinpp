package org.goblinpp.jetbrains.actions;

import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VirtualFile;

import java.util.List;

public final class GoblinRunAction extends GoblinCliAction {
    @Override protected String title() { return "Run"; }
    @Override protected List<String> arguments(Project project, VirtualFile file) {
        return List.of("run", file.getPath());
    }
}
