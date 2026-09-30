package org.goblinpp.jetbrains.actions;

import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VirtualFile;

import java.util.List;

public final class GoblinCompileRunAction extends GoblinCliAction {
    @Override protected String title() { return "Compile and Run"; }
    @Override protected List<String> arguments(Project project, VirtualFile file) {
        return List.of("run", "--compile", file.getPath());
    }
}
