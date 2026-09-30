package org.goblinpp.jetbrains.actions;

import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VirtualFile;

import java.util.List;

public final class GoblinCheckAction extends GoblinCliAction {
    @Override protected String title() { return "Check"; }
    @Override protected List<String> arguments(Project project, VirtualFile file) {
        return List.of("check", file.getPath());
    }
}
