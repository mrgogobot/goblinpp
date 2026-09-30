package org.goblinpp.jetbrains.run;

import com.intellij.execution.actions.ConfigurationContext;
import com.intellij.execution.actions.LazyRunConfigurationProducer;
import com.intellij.execution.configurations.ConfigurationFactory;
import com.intellij.execution.configurations.ConfigurationTypeUtil;
import com.intellij.openapi.project.DumbAware;
import com.intellij.openapi.project.Project;
import com.intellij.openapi.util.Ref;
import com.intellij.openapi.vfs.VirtualFile;
import com.intellij.psi.PsiElement;
import com.intellij.psi.PsiFile;
import org.jetbrains.annotations.NotNull;

public final class GoblinRunConfigurationProducer
        extends LazyRunConfigurationProducer<GoblinRunConfiguration>
        implements DumbAware {
    @Override
    protected boolean setupConfigurationFromContext(
            @NotNull GoblinRunConfiguration configuration,
            @NotNull ConfigurationContext context,
            @NotNull Ref<PsiElement> sourceElement) {
        PsiFile file = psiFile(context);
        VirtualFile virtualFile = file == null ? null : file.getVirtualFile();
        if (!isGoblinFile(virtualFile)) {
            return false;
        }

        configuration.setScriptPath(virtualFile.getPath());
        configuration.setWorkingDirectory(defaultWorkingDirectory(context.getProject(), virtualFile));
        configuration.setGeneratedName();
        sourceElement.set(file);
        return true;
    }

    @Override
    public boolean isConfigurationFromContext(
            @NotNull GoblinRunConfiguration configuration,
            @NotNull ConfigurationContext context) {
        PsiFile file = psiFile(context);
        VirtualFile virtualFile = file == null ? null : file.getVirtualFile();
        return isGoblinFile(virtualFile)
                && pathsEqual(configuration.getScriptPath(), virtualFile.getPath());
    }

    @Override
    public @NotNull ConfigurationFactory getConfigurationFactory() {
        GoblinRunConfigurationType type = ConfigurationTypeUtil.findConfigurationType(
                GoblinRunConfigurationType.class);
        return type.getConfigurationFactories()[0];
    }

    private static PsiFile psiFile(ConfigurationContext context) {
        PsiElement location = context.getPsiLocation();
        return location == null ? null : location.getContainingFile();
    }

    private static boolean isGoblinFile(VirtualFile file) {
        return file != null && "gbl".equalsIgnoreCase(file.getExtension());
    }

    private static String defaultWorkingDirectory(Project project, VirtualFile file) {
        String basePath = project.getBasePath();
        if (basePath != null && !basePath.isBlank()) {
            return basePath;
        }
        VirtualFile parent = file.getParent();
        return parent == null ? "" : parent.getPath();
    }

    private static boolean pathsEqual(String left, String right) {
        try {
            return java.nio.file.Path.of(left).normalize()
                    .equals(java.nio.file.Path.of(right).normalize());
        } catch (RuntimeException error) {
            return left.equals(right);
        }
    }
}
