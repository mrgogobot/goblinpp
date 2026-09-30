package org.goblinpp.jetbrains.actions;

import com.intellij.openapi.project.Project;
import com.intellij.openapi.vfs.VirtualFile;

import java.util.List;

public final class GoblinDoctorAction extends GoblinCliAction {
    @Override protected String title() { return "Doctor"; }
    @Override protected boolean requiresGoblinFile() { return false; }
    @Override protected List<String> arguments(Project project, VirtualFile file) {
        return List.of("doctor", project.getBasePath() == null ? "." : project.getBasePath());
    }
}
