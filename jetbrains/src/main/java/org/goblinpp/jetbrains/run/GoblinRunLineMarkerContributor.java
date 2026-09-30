package org.goblinpp.jetbrains.run;

import com.intellij.execution.lineMarker.ExecutorAction;
import com.intellij.execution.lineMarker.RunLineMarkerContributor;
import com.intellij.icons.AllIcons;
import com.intellij.openapi.project.DumbAware;
import com.intellij.psi.PsiElement;
import com.intellij.psi.PsiWhiteSpace;
import com.intellij.psi.util.PsiTreeUtil;
import org.goblinpp.jetbrains.GoblinFile;
import org.jetbrains.annotations.NotNull;
import org.jetbrains.annotations.Nullable;

public final class GoblinRunLineMarkerContributor
        extends RunLineMarkerContributor implements DumbAware {
    @Override
    public @Nullable Info getInfo(@NotNull PsiElement element) {
        if (!(element.getContainingFile() instanceof GoblinFile)) {
            return null;
        }
        PsiElement first = PsiTreeUtil.getDeepestFirst(element.getContainingFile());
        while (first instanceof PsiWhiteSpace) {
            first = PsiTreeUtil.nextLeaf(first);
        }
        if (element != first) {
            return null;
        }
        return new Info(
                AllIcons.RunConfigurations.TestState.Run,
                ExecutorAction.getActions(0),
                ignored -> "Run " + element.getContainingFile().getName());
    }
}
