package org.goblinpp.jetbrains.actions;

import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VirtualFile;

import java.util.List;

public final class GoblinFreezeAction extends GoblinCliAction {
    @Override protected String title() { return "Freeze"; }
    @Override protected List<String> arguments(Project project, VirtualFile file) {
        return List.of("freeze", file.getPath());
    }
}
